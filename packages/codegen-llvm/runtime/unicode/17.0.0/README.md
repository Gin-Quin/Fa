# Unicode 17.0.0 segmentation data

Sources are the Unicode Character Database 17.0.0:

- https://www.unicode.org/Public/17.0.0/ucd/auxiliary/GraphemeBreakProperty.txt
- https://www.unicode.org/Public/17.0.0/ucd/DerivedCoreProperties.txt
- https://www.unicode.org/Public/17.0.0/ucd/emoji/emoji-data.txt
- https://www.unicode.org/Public/17.0.0/ucd/auxiliary/GraphemeBreakTest.txt

`../../grapheme_tables.py` checks their SHA-256 digests before generating the
runtime table and C conformance cases. Its default operation is offline.
The rules are UAX #29 revision 47:
https://www.unicode.org/reports/tr29/tr29-47.html.

Copyright © 2025 Unicode, Inc. The files and their derived tables are covered
by the Unicode-3.0 license, reproduced in `LICENSE.txt` from
https://www.unicode.org/license.txt.
