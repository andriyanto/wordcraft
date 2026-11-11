# Fuzzing the file readers

Every reader takes files from strangers, so each has a libFuzzer target here:

| Target | Reads |
|---|---|
| `docx_read` | `.docx` packages and Word XML (Flat OPC, Word 2003 XML), then writes them back |
| `doc_read` | Word 97–2003 binary `.doc` |
| `formats_read` | RTF, ODT, flat ODT, HTML, MHT, Markdown, LaTeX, plain text (first byte picks one) |
| `metafile_parse` | WMF and EMF pictures |

```sh
cargo install cargo-fuzz
cargo +nightly fuzz run docx_read -- -max_len=1048576 -rss_limit_mb=4096 -timeout=30
```

Seed a corpus with real files (`fuzz/corpus/<target>/`); for `formats_read`, put the format's
index byte first (0 = rtf, 1 = odt, 2 = fodt, 3 = html, 4 = mht, 5 = md, 6 = tex, 7 = txt).

A finding is a panic, a hang (`-timeout`) or memory past `-rss_limit_mb`. Report it as
`SECURITY.md` says; add the minimised input as a regression test in the reader's own tests.
`corpus/` and `artifacts/` are not committed.
