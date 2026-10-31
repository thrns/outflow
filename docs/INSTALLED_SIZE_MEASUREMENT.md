# Installed Size Measurement

## Purpose and scope

[`scripts/count_installed_bytes.py`](../scripts/count_installed_bytes.py) is a read-only, standard-library Python utility for counting logical bytes beneath a caller-supplied installation directory. It does not package, install, download, inspect file contents, profile memory, or integrate with the native application. It makes no network requests.

The counter supports the size portion of T-005 and the measurement method in [ARCHITECTURE §8.1](ARCHITECTURE.md#81-benchmark-protocol). It does not by itself satisfy AC-01, certify a release, or establish that Outflow fits its size or hardware budgets.

## Usage

Run the synthetic demonstration from the repository root:

```sh
python3 scripts/count_installed_bytes.py --demo
```

It creates an isolated temporary tree containing a 32-byte executable placeholder, a 64-byte model placeholder, and a 16-byte notice, verifies the 112-byte total, then removes the tree. Its output is:

```text
Outflow installed-byte counter ready
Files counted: 3
Total logical bytes: 112
Limit bytes: 950000000
Budget comparison: BELOW LIMIT
Measurement type: SYNTHETIC
Release certification: NOT ASSESSED
```

Measure an explicitly supplied directory with:

```sh
python3 scripts/count_installed_bytes.py --root <installation-directory>
```

The command emits a JSON object with regular-file entries, regular-file bytes, symbolic-link count and logical bytes, aggregate logical bytes, the configured limit, and the strict comparison result. It does not print the supplied absolute path. Errors are controlled and omit filesystem paths. A Python caller can use `count_installed_bytes(root)` and the returned immutable `MeasurementResult`.

## What is counted

Every regular-file directory entry beneath the root is counted by its `stat` logical length. Traversal does not filter by name or extension, so it includes executables, native libraries, model weights, tokenizers, other model assets, static UI files, notices, support files, migrations, and update residue found inside the supplied directory. Hidden files are included. Directory metadata is not included. Files outside the root, including operating-system frameworks, are not included.

Each regular-file directory entry contributes separately. If two names are hard links to the same inode, both names and both reported logical lengths count. This is a conservative logical-size estimate for the directory tree, not a measurement of physically allocated disk blocks. Sparse files, compression, filesystem block rounding, snapshots, and storage deduplication can make allocated disk usage differ.

Symbolic links are counted separately using the link entry's own `lstat` length. They are never descended during traversal. A link must resolve to an existing target within the supplied root; broken, escaping, or unverifiable links cause the measurement to fail. The root itself may not be a symbolic link. Links to an in-root ancestor are counted once and do not form traversal cycles because link targets are not traversed. The aggregate `total_logical_bytes` used for the budget includes regular-file bytes plus symbolic-link entry bytes; the result also reports both components separately.

Unexpected special entries such as FIFOs, sockets, and device nodes cause a controlled error. Directory read failures and inaccessible metadata also fail the measurement rather than returning a partial total.

## Release threshold and interpretation

The configured threshold is exactly **950,000,000 decimal bytes**. A result is below the limit only when its aggregate logical byte count is strictly less than that value. A count of 949,999,999 passes the comparison; 950,000,000 and 950,000,001 do not.

For release evidence, measure the complete application after running the actual release installer on each release target. The measured root must contain the installed executable, native runtime assets, the selected bundled ASR model and all its tokenizer/configuration assets, static UI, notices, support files, migrations, and any update residue in that application directory. Follow [ARCHITECTURE §8.1](ARCHITECTURE.md#81-benchmark-protocol) for the broader benchmark protocol. An installer cache and operating-system frameworks outside the installation root are excluded from this count.

The demonstration uses synthetic placeholders and reports `Release certification: NOT ASSESSED`. A synthetic result or a measurement of an arbitrary directory is not evidence that Outflow meets AC-01. AC-01 also requires offline clean installation and first dictation, intact bundled models, and an actual installed-size result on each release target. The project currently has no complete installed release to certify through this utility.

## Platform differences and limitations

This implementation uses Python's standard-library directory and metadata APIs. Symlink permissions and behavior, path resolution, metadata semantics, access-control rules, and filesystem support vary across macOS, Windows, and Linux. The current verification has run on macOS 26.6.2 arm64; native Windows and Linux behavior remains unverified.

The utility does not lock the tree or claim protection against concurrent filesystem changes. If files, directories, or links change while it runs, the result can fail or reflect a mixed snapshot. It also does not measure installer caches, files outside the selected root, filesystem allocation, or peak memory. Reading directory entries may affect access-time metadata according to the operating system and filesystem policy, although the utility issues no content writes and does not alter file contents.

## Tests and troubleshooting

Run the focused tests or all script tests from the repository root:

```sh
python3 -m unittest scripts.tests.test_count_installed_bytes -v
python3 -m unittest discover -s scripts/tests -v
```

- **Root missing or not a directory:** supply an existing directory explicitly with `--root`.
- **Symbolic-link error:** remove or repair broken links and ensure every link target resolves within the selected root. The utility does not follow links to include their targets.
- **Special filesystem entry:** the selected tree contains a socket, device, FIFO, or other non-regular non-directory entry; this counter deliberately fails instead of guessing how to size it.
- **Directory or metadata access error:** rerun with read/search permission for every directory and metadata access for each entry. The command does not print the inaccessible path.
- **At or above the limit:** the supplied tree is not below the strict threshold. Inspect the actual release contents and model assets; do not infer feasibility from the synthetic demo.
- **Different result on another operating system:** check symlink permissions and filesystem behavior, then repeat on the native release target. This tool does not normalize platform storage allocation semantics.
