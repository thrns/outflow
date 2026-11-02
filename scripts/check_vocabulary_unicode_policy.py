#!/usr/bin/env python3
"""Show a development-only Unicode vocabulary comparison reference.

This script compares synthetic strings only. It does not read or write files,
make network requests, persist vocabulary, or print spoken phrase contents.
It is not production application code and does not establish conformance to a
particular Unicode release unless the runtime data version is verified.
"""

from __future__ import annotations

import argparse
import unicodedata
from collections.abc import Sequence


def canonical_caseless_key(value: str) -> str:
    """Return the NFD(default full case fold(NFD(value))) comparison key.

    This mirrors Unicode canonical caseless matching as a standard-library
    development reference. The Python runtime's Unicode data version controls
    its behavior. The function returns a derived key and never changes the
    input string or performs vocabulary replacement.
    """
    decomposed = unicodedata.normalize("NFD", value)
    folded = decomposed.casefold()
    return unicodedata.normalize("NFD", folded)


def comparison_is_equal(first: str, second: str) -> bool:
    """Compare two strings using the candidate reference key."""
    return canonical_caseless_key(first) == canonical_caseless_key(second)


def unicode_data_version() -> str:
    """Return the Unicode Character Database version used by this Python."""
    return unicodedata.unidata_version


_DEMO_CASES = (
    ("ASCII case variation", "rust", "Rust"),
    ("Non-ASCII case variation", "CAF\u00c9", "caf\u00e9"),
    ("German sharp-s variation", "stra\u00dfe", "STRASSE"),
    ("Canonical accent variation", "\u00e9", "e\u0301"),
    ("Greek sigma variation", "\u03a3", "\u03c3"),
    ("Greek final sigma variation", "\u03c3", "\u03c2"),
    ("Multiword case variation", "North Parkade", "north parkade"),
    ("Trailing whitespace distinction", "north parkade", "north parkade "),
    ("Turkish dotless-I distinction", "I", "\u0131"),
    ("Turkish dotted-I distinction", "\u0130", "i"),
    ("Compatibility numeral distinction", "\u2460", "1"),
    ("Cross-script confusable distinction", "a", "\u0430"),
)


def _run_demo() -> None:
    print("Outflow vocabulary Unicode policy review")
    print("Current Rust policy: LOWERCASE-ONLY (PROVISIONAL)")
    print("Candidate policy: CANONICAL CASELESS MATCHING (D145)")
    print(f"Python Unicode data version: {unicode_data_version()}")
    print()

    for label, first, second in _DEMO_CASES:
        result = "EQUAL" if comparison_is_equal(first, second) else "DIFFERENT"
        print(f"{label}: {result}")

    print()
    print("Production Rust validator: UNCHANGED")
    print("Candidate production policy: NOT APPROVED")
    print("Vocabulary persistence: NOT IMPLEMENTED")
    print("Text replacement: NOT IMPLEMENTED")


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--demo",
        action="store_true",
        help="compare the fixed synthetic Unicode examples",
    )
    arguments = parser.parse_args(argv)
    if not arguments.demo:
        parser.error("choose --demo to run the synthetic comparison demonstration")

    _run_demo()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
