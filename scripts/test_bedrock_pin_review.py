"""Synthetic corruption tests for the independently signed Bedrock pin review."""
import hashlib, io, unittest
from review_bedrock_pin import PAGE, ENTRIES, level_sizes, verify_tree


class PinReviewTests(unittest.TestCase):
    def fixture(self, count=171):
        data = [bytes([i % 256]) * PAGE for i in range(count)]
        sizes = level_sizes(count)
        levels = []
        child = data
        for size in reversed(sizes):
            pages = [bytearray(PAGE) for _ in range(size)]
            for i, page in enumerate(child):
                digest = hashlib.sha256(page).digest()
                length = 20 if not levels else 24
                pages[i // ENTRIES][i % ENTRIES * 24:i % ENTRIES * 24 + length] = digest[:length]
            child = [bytes(page) for page in pages]
            levels.insert(0, child)
        tree = b''.join(page for level in levels for page in level)
        return tree + b''.join(data), hashlib.sha256(tree[:PAGE]).digest(), count

    def test_all_nodes_and_content_pages_match_signed_root(self):
        data, root, count = self.fixture()
        pages, end = verify_tree(io.BytesIO(data), 0, count, root)
        self.assertEqual(pages, 3)
        self.assertEqual(end, len(data))

    def test_root_node_leaf_and_truncated_content_rejected(self):
        data, root, count = self.fixture()
        for offset in [0, PAGE + 5, 3 * PAGE + 7, len(data)-1]:
            with self.subTest(offset=offset):
                damaged = bytearray(data)
                damaged[offset] ^= 1
                with self.assertRaises(ValueError):
                    verify_tree(io.BytesIO(damaged), 0, count, root)
        with self.assertRaises(ValueError):
            verify_tree(io.BytesIO(data[:-1]), 0, count, root)

    def test_each_tree_depth_and_run_boundary(self):
        for count in [1, 170, 171, 28901]:
            with self.subTest(count=count):
                data, root, count = self.fixture(count)
                _, end = verify_tree(io.BytesIO(data), 0, count, root)
                self.assertEqual(end, len(data))

    def test_untrusted_or_invalid_tree_root_rejected(self):
        data, root, count = self.fixture()
        with self.assertRaises(ValueError):
            verify_tree(io.BytesIO(data), 0, count, bytes(32))
        for count in [0, ENTRIES**4+1]:
            with self.assertRaises(ValueError):
                level_sizes(count)


if __name__ == '__main__':
    unittest.main()
