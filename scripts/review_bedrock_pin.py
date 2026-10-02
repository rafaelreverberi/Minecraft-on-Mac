#!/usr/bin/env python3
"""Engineering-only review of a licensed, encrypted Bedrock container.

No decryption or installation. RSA-PSS/SHA256 authenticates the header using the
retail public key independently recovered from our digest-pinned official GDK.
The signed root authenticates all hash-table nodes and all stored content pages.
Release installation still requires the reviewed full SHA256; it has no bypass.
"""
from pathlib import Path
import argparse, base64, hashlib, json, struct, subprocess, tempfile, uuid

PAGE = 4096
ENTRIES = 170
PUBLIC_KEY_SHA256 = '183f0ae05431e4ad91554e88946967c872997227dbe6c85116f5fd2fd2d1229e'


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read_exact(source, offset, length):
    source.seek(offset)
    data = source.read(length)
    require(len(data) == length, 'Truncated container')
    return data


def level_sizes(data_pages):
    require(0 < data_pages <= ENTRIES ** 4, 'Unsupported page count')
    levels = []
    while True:
        data_pages = (data_pages + ENTRIES - 1) // ENTRIES
        levels.append(data_pages)
        if data_pages == 1:
            return list(reversed(levels))


def verify_tree(source, tree_start, data_pages, signed_root):
    sizes = level_sizes(data_pages)
    total_tree_pages = sum(sizes)
    tree = read_exact(source, tree_start, total_tree_pages * PAGE)
    require(hashlib.sha256(tree[:PAGE]).digest() == signed_root, 'Signed tree root mismatch')
    offsets = [0]
    for size in sizes[:-1]:
        offsets.append(offsets[-1] + size)
    for level in range(1, len(sizes)):
        parent = offsets[level - 1]
        child = offsets[level]
        for index in range(sizes[level]):
            entry = (parent + index // ENTRIES) * PAGE + index % ENTRIES * 24
            digest = hashlib.sha256(tree[(child + index)*PAGE:(child + index + 1)*PAGE]).digest()
            require(digest[:24] == tree[entry:entry + 24], 'Hash-table node mismatch')
    leaves = offsets[-1]
    data_start = tree_start + total_tree_pages * PAGE
    source.seek(data_start)
    for index in range(data_pages):
        page = source.read(PAGE)
        require(len(page) == PAGE, 'Truncated content page')
        entry = (leaves + index // ENTRIES) * PAGE + index % ENTRIES * 24
        require(hashlib.sha256(page).digest()[:20] == tree[entry:entry + 20], 'Stored content page mismatch')
    return total_tree_pages, data_start + data_pages * PAGE


def verify_signature(header, public_key):
    require(hashlib.sha256(public_key).hexdigest() == PUBLIC_KEY_SHA256, 'Unreviewed public key')
    magic, bits, esize, nsize, p1, p2 = struct.unpack_from('<6I', public_key)
    require((magic, bits, esize, nsize, p1, p2) == (0x31415352, 4096, 3, 512, 0, 0), 'Invalid public-only RSA blob')
    require(len(public_key) == 539, 'Invalid public key size')

    def tlv(tag, data):
        length = len(data)
        size = length.to_bytes((length.bit_length()+7)//8, 'big')
        return bytes([tag]) + (bytes([length]) if length < 128 else bytes([0x80 + len(size)]) + size) + data

    def integer(data):
        return tlv(2, (b'\0' if data[0] & 0x80 else b'') + data)

    exponent, modulus = public_key[24:27], public_key[27:]
    der = tlv(0x30, integer(modulus) + integer(exponent))
    pem = b'-----BEGIN RSA PUBLIC KEY-----\n' + base64.encodebytes(der) + b'-----END RSA PUBLIC KEY-----\n'
    with tempfile.TemporaryDirectory(prefix='mml-bedrock-public-review-') as directory:
        root = Path(directory)
        (root/'key.pem').write_bytes(pem)
        (root/'signature').write_bytes(header[:512])
        (root/'header').write_bytes(header[512:])
        result = subprocess.run(['openssl', 'dgst', '-sha256', '-sigopt', 'rsa_padding_mode:pss',
                                 '-sigopt', 'rsa_pss_saltlen:32', '-verify', str(root/'key.pem'),
                                 '-signature', str(root/'signature'), str(root/'header')],
                                capture_output=True, check=False)
        require(result.returncode == 0, 'Microsoft retail header signature rejected')


def review(package, public_key, content_id, size):
    require(package.stat().st_size == size, 'Service size mismatch')
    with package.open('rb') as source:
        header = read_exact(source, 0, PAGE)
        verify_signature(header, public_key.read_bytes())
        require(header[512:520] == b'msft-xvd', 'Invalid container')
        require(uuid.UUID(bytes_le=header[544:560]) == uuid.UUID(content_id), 'Authenticated content ID mismatch')
        flags = struct.unpack_from('<I', header, 0x208)[0]
        require(flags == 0x41, 'Unsupported container flags')
        require(struct.unpack_from('<I', header, 0x280)[0] == 0, 'Unsupported dynamic container')
        require(tuple(reversed(struct.unpack_from('<4H', header, 0x3bc))) == (1, 26, 5203, 0), 'Unsupported signed version')
        drive_size = struct.unpack_from('<Q', header, 0x218)[0]
        embedded, user, xvc, dynamic = struct.unpack_from('<4I', header, 0x288)
        mutable = header[0x470]
        require(embedded == dynamic == 0 and mutable == 1, 'Unsupported layout')
        require(read_exact(source, PAGE, PAGE*2) == bytes(PAGE*2), 'Unexpected reserved header data')
        # This baseline's mutable region-presence page is canonical and unused by
        # extraction: all nine downloaded regions present, remaining bytes zero.
        require(read_exact(source, PAGE*3, PAGE) == b'\3'*9 + bytes(PAGE-9), 'Unexpected mutable region-presence data')
        data_pages = sum((length+PAGE-1)//PAGE for length in [user, xvc, dynamic, drive_size])
        tree_pages, end = verify_tree(source, PAGE*(3+mutable), data_pages, header[0x240:0x260])
        require(end == size, 'Unverified trailing container bytes')
    with package.open('rb') as source:
        digest = hashlib.file_digest(source, 'sha256').hexdigest()
    return {'schema': 1, 'gameId': 'bedrock', 'version': '1.26.5203.0', 'contentId': content_id,
            'size': size, 'sha256': digest, 'headerSignature': 'Microsoft retail RSA-PSS-SHA256',
            'publicKeySHA256': PUBLIC_KEY_SHA256, 'hashTreePages': tree_pages,
            'verifiedContentPages': data_pages, 'reservedAndMutablePages': 'canonical baseline',
            'decrypted': False}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--package', type=Path, required=True)
    parser.add_argument('--public-key', type=Path, required=True)
    parser.add_argument('--content-id', required=True)
    parser.add_argument('--size', type=int, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = review(args.package, args.public_key, args.content_id, args.size)
    with args.output.open('x') as output:
        json.dump(result, output, indent=2)
        output.write('\n')
    print(json.dumps(result))
