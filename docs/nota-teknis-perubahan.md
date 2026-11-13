# Nota teknis: perubahan repo ini dibanding repo asal

Nota ini mencatat apa yang diubah di `andriyanto/wordcraft` sejak disalin dari proyek asalnya, mengapa, bagaimana rancangannya, dan bagaimana hasilnya dibanding titik awal. Ditulis 13 November 2026.

## 1. Ruang lingkup

| | |
|---|---|
| Repo asal | `storytold/wordcraft` |
| Titik awal | commit `118eb8a` (11 November 2026), versi 0.15.1 |
| Repo ini | `andriyanto/wordcraft`, dikembangkan mandiri sejak titik awal |
| Periode kerja | 11–13 November 2026, PR #6 sampai #29 |

Dua batasan cara membaca nota ini:

- **"Repo asal" berarti keadaan pada commit `118eb8a`.** Perubahan yang dibuat proyek asal setelah itu tidak diperiksa dan tidak dibandingkan.
- **Angka kinerja diukur pada build release dengan dokumen sintetis** (paragraf teks biasa) di mesin pengembangan. Angka itu menunjukkan perbandingan sebelum dan sesudah, bukan jaminan untuk dokumen nyata. Alat ukurnya ada di luar repo.

Nomor versi belum diubah: masih 0.15.1, sama dengan repo asal.

## 2. Ringkasan perbandingan

| Aspek | Repo asal (`118eb8a`) | Repo ini |
|---|---|---|
| Ukuran `Paragraph` di memori | 928 byte | 104 byte |
| Ukuran `Run` di memori | 376 byte | 16 byte |
| Buka 999.000 paragraf kosong | 1.314 MB | 522 MB |
| Memori layout, 993 halaman | 359 MB | 259 MB |
| Memori puncak baca DOCX, 20.000 paragraf berformat | 182 MB | 117 MB |
| Relayout per ketukan, 993 halaman | 56 ms | 11 ms (sebelum paginasi inkremental) |
| Relayout per ketukan, 2.837 halaman | 161 ms | 14–19 ms |
| Hitungan status bar per ketukan, 2.837 halaman | 85 ms | 2,3 ms |
| DOCX 27 KB berisi 2 juta paragraf | memakai 2,6 GB, proses mati | ditolak dengan pesan galat |
| Server MCP tanpa antarmuka | akses seluruh berkas | terbatas pada folder kerja |
| Sumber pembaruan aplikasi | rilis `storytold/wordcraft` | rilis `andriyanto/wordcraft` |
| Jendela dokumen kedua di GPU Adreno | terdaftar tetapi tidak tampil | tampil (lewat OpenGL) |
| Setelah File › Export | tetap di halaman Export, tanpa tanda | kembali ke dokumen, ada pemberitahuan |
| Pratinjau langsung di ribbon Home | tidak ada | font, ukuran, warna, gaya (PR #28) |

Keluaran dokumen tidak berubah: untuk setiap optimasi, hasil render dan konversi dibandingkan byte demi byte dengan build sebelum perubahan (lihat bagian 9).

## 3. Keamanan dan ketahanan (PR #6)

Hasil audit awal atas kode dan dependensi.

| Temuan | Perbaikan |
|---|---|
| Berkas DOCX kecil bisa menghabiskan memori: 27 KB berisi 2 juta `<w:p/>` memakai 2,6 GB | Anggaran `MAX_CONTENT` = 1.000.000 paragraf dan run per berkas, termasuk dokumen bersarang; lebih dari itu ditolak |
| Objek OLE tanpa gambar pratinjau dibuang saat dibaca | Objek dan datanya dipertahankan, ditampilkan sebagai kotak kosong, ditulis kembali saat simpan |
| Server MCP tanpa antarmuka bisa membaca dan menulis berkas mana pun | Bawaan dibatasi ke folder tempat ia dijalankan; akses penuh hanya dengan `--allow-all-files` |
| Dependensi dengan advisory | `tokio` 1.54.1, `async-executor` 1.14.1, kemudian `rustix` 1.1.6 (PR #9) |
| Tidak ada pemeriksaan advisory berkala | Workflow `audit` harian dan kebijakan di `.cargo/audit.toml` |
| Workflow bersama dirujuk lewat `@main` | Dikunci ke hash commit |

Tambahan: empat target fuzzing libFuzzer di `fuzz/` (docx, doc, format lain, metafile). Target itu baru dikompilasi, belum pernah dijalankan.

Yang tersisa: sebelum menolak berkas bom, pembaca masih memakai 1–2 GB untuk pohon XML. Advisory `zip` diabaikan dengan alasan tercatat, karena fungsi yang terkena tidak dipanggil.

## 4. Memori model dokumen: properti dipakai bersama (PR #7)

**Masalah.** Setiap paragraf, run, tabel, dan sel menyimpan salinan utuh properti formatnya, padahal sebagian besar isinya sama atau kosong.

**Rancangan.** `Shared<T>` di `crates/doc/src/shared.rs`:

- Nilai disimpan di balik `Arc`. Membaca lewat `Deref`; mengubah lewat `DerefMut`, yang menyalin dulu bila nilainya sedang dipakai bersama.
- Nilai bawaan (kosong) adalah satu alokasi untuk seluruh proses.
- Nilai lain dicari di 16 nilai terakhir per thread, sehingga paragraf berurutan dengan format sama memakai satu salinan.
- Serialisasi sama persis dengan nilai yang dibungkus, jadi format JSON tidak berubah.

Dipakai untuk `ParaProps`, `CharProps`, `CellProps`, dan `TableProps`.

| Struktur | Sebelum | Sesudah |
|---|---|---|
| Paragraf | 928 byte | 104 byte |
| Run | 376 byte | 16 byte |
| Tabel | 368 byte | 88 byte |
| Sel | 256 byte | 64 byte |

| Membuka berkas | Sebelum | Sesudah |
|---|---|---|
| 999.000 paragraf kosong | 1.314 MB | 522 MB |
| 200.000 paragraf kosong | 324 MB | 165 MB |
| 400.000 sel tabel | 490 MB | 367 MB |

Batasnya: setelah banyak paragraf diedit satu per satu, masing-masing kembali punya salinan sendiri, sehingga memori naik lagi. Hasilnya tetap benar.

## 5. Biaya per ketukan

### 5.1 Hitungan status bar (PR #10)

Sebelumnya tiap ketukan menghitung ulang kata seluruh dokumen. `CountCache` di `crates/doc/src/counts.rs` menyimpan hitungan per blok tingkat atas berdasarkan alamat bloknya, lalu hanya menghitung ulang blok yang berubah. Paragraf bernomor tetap dihitung tiap kali, karena labelnya bergantung pada paragraf sebelumnya.

| | 284 halaman | 993 halaman | 2.837 halaman |
|---|---|---|---|
| Sebelum | 7,1 ms | 28,5 ms | 85 ms |
| Sesudah | 0,08 ms | 0,57 ms | 2,3 ms |

Dokumen yang isinya satu tabel raksasa tidak terbantu, karena satu tabel adalah satu blok.

### 5.2 Relayout tanpa mengubah algoritma (PR #10, #12)

Profil menunjukkan pekerjaan yang diulang untuk setiap paragraf pada setiap pass. Empat di antaranya dihilangkan:

| Pekerjaan berulang | Perbaikan |
|---|---|
| Resolve gaya dua paragraf untuk memeriksa grup bingkai | Jawaban diingat berdasarkan alamat properti bersama |
| Penempatan objek menelusuri semua cluster | Dilewati bila paragraf tidak punya objek |
| Resolve gaya paragraf sampai tiga kali | Memo per alamat properti selama style sheet sama (`crates/layout/src/memo.rs`) |
| Hashing kunci cache dengan SipHash | Hasher cepat untuk peta internal layout |

| Relayout per ketukan | 284 halaman | 993 halaman | 2.837 halaman |
|---|---|---|---|
| Repo asal | 13,9 ms | 56 ms | 161 ms |
| Setelah PR #10 | 4,6 ms | 21 ms | 73 ms |
| Setelah PR #12 | 3,0 ms | 11 ms | 55 ms |

Memo berdasarkan alamat hanya mungkin karena bagian 4: properti yang sama kini benar-benar satu objek.

Yang dicoba dan dibatalkan: membebaskan layout lama di thread terpisah. Hasil ukurnya lebih lambat, jadi tidak dipakai.

## 6. Memori layout (PR #11)

Dari 359 MB layout dokumen 993 halaman, 256 MB adalah daftar *cluster* (satu entri per karakter), dan hampir sepertiganya ruang kosong.

- Ruang cadangan daftar cluster, glyph, dan posisi baris dilepas setelah paragraf selesai ditata (`ParaLayout::release_spare_room`).
- `Cluster` dipadatkan dari 64 menjadi 44 byte: rentang byte dan jenisnya disimpan dalam 32 bit dan dibaca lewat `start()`, `end()`, `kind()`.

| Dokumen | Sebelum | Sesudah |
|---|---|---|
| 284 halaman | 118 MB | 91 MB |
| 993 halaman | 359 MB | 259 MB |
| 2.837 halaman | 981 MB | 701 MB |

Kecepatan tidak berubah.

## 7. Pembaca DOCX (PR #13)

Pembaca membangun pohon XML seluruh part sebelum mengubahnya menjadi dokumen. Dua pemborosan dihilangkan di `crates/docx/src/xml.rs`:

- Daftar anak dan atribut dipangkas begitu elemennya selesai dibaca.
- `xml::Name`: satu salinan untuk tiap nama, dipakai bersama oleh semua elemen dan atribut bernama sama.

| Dokumen (memori puncak) | Sebelum | Sesudah |
|---|---|---|
| 20.000 paragraf berformat (XML 15 MB) | 182 MB | 117 MB |
| 900.000 run dalam satu paragraf | 891 MB | 379 MB |
| 400.000 sel tabel | 346 MB | 136 MB |
| 999.000 paragraf kosong | 240 MB | 201 MB |

Solusi tuntasnya adalah membaca secara streaming tanpa pohon, yang berarti menulis ulang pembaca. Itu belum dikerjakan.

## 8. Paginasi inkremental (PR #14, #15, #16)

Ini perubahan rancangan terbesar. Di repo asal, setiap ketukan menata ulang seluruh dokumen dari blok pertama.

### 8.1 Gagasan

Edit tidak mengubah apa pun sebelum titik edit, dan sering kali tidak mengubah batas halaman sesudahnya. Jadi sebuah pass cukup:

1. **mulai** dari titik aman terakhir sebelum edit, lalu
2. **berhenti** begitu keadaannya kembali persis seperti pass sebelumnya.

Syarat mutlaknya: hasilnya harus sama dengan penataan dari nol.

### 8.2 Tahapan

| Tahap | PR | Isi | Dampak kecepatan |
|---|---|---|---|
| 0 | #14 | Alat uji: tiap layout dibandingkan dengan layout dari nol | Tidak ada |
| 1 | #14 | State paginasi berdiri sendiri dan bisa disimpan | Tidak ada |
| 2 | #15 | Mulai dari mark sebelum titik edit | Edit di akhir dokumen |
| 3 | #16 | Berhenti di mark saat state kembali sama | Edit di mana pun |
| 4 | belum | Sisip dan hapus paragraf juga bisa berhenti lebih awal | — |

### 8.3 Komponen

| Komponen | Letak | Peran |
|---|---|---|
| `verify_against_full`, `same_layout` | `crates/layout/src/verify.rs` | Membandingkan hasil dengan layout dari nol, halaman per halaman |
| `replay_blocks` | `crates/layout/src/verify.rs` | Menata tiap blok dua kali dari checkpoint; membuktikan checkpoint lengkap |
| `Checkpoint` | `crates/layout/src/pass.rs` | State paginasi pada batas blok, termasuk halaman yang sedang diisi |
| `Mark` | `crates/layout/src/resume.rs` | Checkpoint yang disimpan tiap 8 halaman untuk pass berikutnya |
| `Kept`, `Given` | `crates/layout/src/resume.rs` | Hasil pass terakhir, dan semua masukan selain isi dokumen |
| `Tail` | `crates/layout/src/resume.rs` | Sisa hasil pass terakhir, diambil saat pass berhenti lebih awal |
| `Items` | `crates/layout/src/placed.rs` | Daftar item halaman yang dibagi sampai diubah, agar halaman lama tidak disalin |

### 8.4 Kapan sebuah mark boleh dipakai untuk mulai

Semua ini harus tidak berubah sejak pass sebelumnya:

- blok-blok sebelum mark, ditambah 12 blok sesudahnya yang "diintip" paginasi (jarak antarparagraf, border gabungan, rantai keep-with-next);
- semua masukan selain isi dokumen: opsi tampilan, style, properti section, story lain (catatan kaki, header, footer), settings, penomoran catatan kaki.

Blok dibandingkan berdasarkan alamatnya. Itu sah karena blok yang sedang dipegang bersama tidak pernah diubah di tempat, hanya diganti salinannya.

### 8.5 Kapan sebuah pass boleh berhenti

- Jumlah blok dan batas section sama.
- Blok mulai dua sebelum mark sampai akhir dokumen adalah blok yang sama.
- State saat tiba di mark sama persis dengan state pass sebelumnya di mark itu: seluruh nilai page builder, counter, dan halaman yang sedang diisi.

### 8.6 Kapan pass tetap berjalan penuh

- Teks catatan kaki, header, atau footer diedit.
- Dokumen punya text box bertaut atau gambar dengan wrap mengikuti kontur.
- Section yang kolomnya diseimbangkan.
- Tampilan Web, Draft, dan Outline.
- Dokumen yang butuh pass ulang: `NUMPAGES` di badan dokumen, float di atas jangkarnya, catatan kaki bernomor per halaman.
- Setiap 256 pass inkremental berturut-turut, satu pass penuh dijalankan agar cache paragraf bisa dibersihkan.

### 8.7 Hasil (2.837 halaman)

| Per ketukan | Sebelum tahap 2 | Tahap 2 | Tahap 3 |
|---|---|---|---|
| Mengetik di awal dokumen | 62–65 ms | 64–67 ms | 14–19 ms |
| Mengetik di akhir dokumen | 64 ms | 18 ms | 15–19 ms |

Sisa sekitar 15 ms bukan lagi paginasi, melainkan pekerjaan per halaman sesudahnya: indeks baris sekitar 9 ms dan header/footer sekitar 3 ms.

### 8.8 Batas yang diketahui

- Edit yang menggeser batas halaman tetap menata ulang sampai batas halaman kembali sama, yang bisa berarti sampai akhir dokumen.
- Sisip atau hapus paragraf tidak bisa berhenti lebih awal, karena item halaman mencatat nomor bloknya (tahap 4).
- Perbandingan state memakai keluaran `Debug` tiap nilai; kontur gambar tercetak sebagai digest 64-bit.
- Batas "dua blok sebelum mark" adalah margin pengaman tanpa tes yang gagal bila margin itu dihapus.

## 9. Metode verifikasi

| Metode | Dipakai untuk | Apa yang dibuktikan |
|---|---|---|
| Keluaran CLI lama vs baru, byte demi byte | Semua optimasi | Render, jumlah halaman, dan konversi (DOCX, ODT, RTF, HTML, TXT, JSON) tidak berubah; terakhir 136 dari 136 identik |
| Tes edit acak dengan pembanding | Paginasi inkremental | Setelah tiap edit, layout sama dengan layout dari nol |
| Mode replay (`WORDCRAFT_LAYOUT_REPLAY=1`) | Tahap 1 | Checkpoint memuat seluruh state |
| Mode resume (`WORDCRAFT_LAYOUT_RESUME=1`) | Tahap 2 dan 3 | Seluruh tes layout tetap benar bila tiap layout diulang dari mark di tengah |
| Bug yang sengaja ditanam | Tahap 0, 2, 3 | Tesnya benar-benar menangkap kesalahan |
| Berkas rusak hasil mutasi (85 berkas) | Pembaca DOCX | Tidak crash dan tidak hang |

Kegagalan tes yang ada di lingkungan pengembangan dan bukan akibat perubahan ini: tiga tes hifenasi layout, satu tes paket bahasa engine, dan satu tes glif UI. Penyebabnya data bahasa dan font yang tidak ada di mesin itu; ketiganya lulus di CI.

## 10. Perubahan platform dan antarmuka

| PR | Masalah | Perubahan | Status uji di perangkat |
|---|---|---|---|
| #24 | Popup pembaruan tiap aplikasi dibuka | Sumber pembaruan dialihkan ke rilis repo ini | Belum dikonfirmasi |
| #25 | Di laptop Snapdragon, jendela dokumen kedua tidak tampil | OpenGL dipakai bila satu-satunya GPU adalah Qualcomm Adreno | Dikonfirmasi pemilik repo |
| #26 | Dialog Open/Save memajukan jendela pertama | Dialog hanya ditempelkan ke jendela pertama bila jendela itu yang meminta | Dikonfirmasi pemilik repo |
| #27 | Tidak ada tanda setelah ekspor; jendela tertinggal di halaman Export | Kembali ke dokumen dan menampilkan pemberitahuan | Belum dikonfirmasi |
| #28 | Daftar di ribbon tidak punya pratinjau langsung | Pratinjau untuk font, ukuran, warna, dan gaya di tab Home | Belum; PR masih terbuka saat nota ditulis |
| #29 | About hanya menyebut proyek asal | Tautan repo ini, disclaimer, kontributor, dan model ditambahkan | Belum; PR masih terbuka saat nota ditulis |

Catatan rancangan:

- **#25 adalah jalan memutar.** Penyebab di jalur DirectX 12 pada driver Adreno belum diketahui. Bila mesin tidak punya OpenGL, aplikasi tetap memakai DirectX 12.
- **#26 punya konsekuensi.** Dialog yang diminta dari jendela kedua tidak modal terhadap jendela itu. Menempelkannya ke jendela yang tepat butuh handle jendela per dokumen, yang tidak tersedia tanpa kode `unsafe` (dilarang di workspace ini).
- **Pembaruan otomatis di macOS akan ditolak**, karena pustaka pembaruan mensyaratkan tanda tangan Apple milik proyek asal.

## 11. CI dan dependensi

| Kejadian | Tindakan |
|---|---|
| CI merah di semua workflow | Penyebabnya penagihan Actions pada repo privat; repo dijadikan publik |
| Dua galat clippy lama di modul clipboard Linux | Diperbaiki (PR #8) |
| Bump `harfrust` 0.15 dan `skrifa` 0.49 (PR #18) membuat `main` tidak bisa dikompilasi | Di-revert (PR #23); migrasi `crates/fonts` ke API baru belum dikerjakan |
| Bump `vello_cpu` 0.3 (PR #19) gagal kompilasi | Ditutup; butuh migrasi kode |
| Rust stabil naik ke 1.100; clippy menolak lima loop di kode tes | Loop ditulis ulang (bagian dari PR #24) |

## 12. Yang masih bergantung pada proyek asal

- Pustaka bersama `storytold/craft-libs` (jendela, pembaruan, cetak, i18n) dan fork `egui` milik proyek asal.
- Workflow CI dan rilis bersama `storytold/craft-common-actions`.
- Katalog terjemahan dan unduhan paket bahasa dari `storytold/craft-i18n`.
- Tautan situs dan Discord di jendela About, serta info aplikasi di pemasang Windows.

## 13. Penyimpangan dari aturan bawaan repo

Berkas `AGENTS.md` berisi aturan kerja yang diwarisi dari proyek asal. Tiga hal yang dikerjakan di repo ini tidak sejalan dengannya, dan dicatat di sini agar diputuskan secara sadar:

| Aturan di `AGENTS.md` | Yang dilakukan di repo ini |
|---|---|
| `contributors/contributors.json` tidak boleh diedit tangan; nama tidak boleh diambil dari konfigurasi git | PR #29 menambah baris ke berkas itu dengan tangan dan memakai nama dari commit, atas permintaan pemilik repo |
| PR tidak menyebut bahwa agen yang menulisnya | Commit dan PR di repo ini mencantumkan Claude sebagai rekan penulis |
| Workflow memanggil workflow bersama di `@main`; CI tidak diubah lewat PR fitur | Workflow dikunci ke hash commit (PR #6) dan ditambah workflow `audit` |

Aturan itu ditulis untuk proyek asal. Karena repo ini dikembangkan mandiri, `AGENTS.md` sebaiknya disesuaikan agar isinya mencerminkan cara kerja yang memang dipakai di sini.

## 14. Rilis dan pemasangan

- Belum ada rilis resmi dari repo ini. Build uji untuk Windows ARM64 dibuat lewat workflow "Windows ARM64" dan diunduh dari artifact, yang kedaluwarsa dalam 7 hari.
- Pemasang tidak ditandatangani, sehingga Windows SmartScreen memperingatkan. Penandatanganan ditunda.
- Workflow rilis memakai sertifikat penandatanganan yang tidak ada di repo ini; belum diketahui apakah ia berjalan tanpa itu.

## 15. Pekerjaan lanjutan yang sudah teridentifikasi

| Pekerjaan | Manfaat |
|---|---|
| Pekerjaan per halaman setelah paginasi (indeks baris, header/footer) dibuat inkremental | Memangkas sisa sekitar 15 ms per ketukan |
| Tahap 4 paginasi inkremental | Sisip dan hapus paragraf berhenti lebih awal |
| Migrasi `crates/fonts` ke `harfrust` 0.15 / `skrifa` 0.49, dan render ke `vello_cpu` 0.3 | Pembaruan dependensi yang tertunda bisa masuk |
| Pembaca DOCX streaming | Menghilangkan pohon XML di memori |
| Jendela kedua tersembunyi pada pembukaan pertama setelah pemasangan | Dugaan: aplikasi dijalankan dari pemasang; belum dibuktikan |
| Menjalankan target fuzzing | Target sudah ada, belum pernah dijalankan |
| Audit `gridcraft` | Belum dimulai |

## 16. Daftar PR

| PR | Judul | Status |
|---|---|---|
| #6 | Perbaikan audit: batas memori DOCX, OLE tanpa pratinjau, MCP terbatas, audit dependensi | Merged |
| #7 | Properti format dipakai bersama | Merged |
| #8 | Dua galat clippy di modul clipboard Linux | Merged |
| #9 | `rustix` 1.1.6 agar workflow audit hijau | Merged |
| #10 | Optimasi per ketukan: relayout dan hitungan status bar | Merged |
| #11 | Memori layout −28% | Merged |
| #12 | Relayout per ketukan: memo gaya dan hasher cepat | Merged |
| #13 | Pembaca DOCX: memori puncak −36% sampai −60% | Merged |
| #14 | Paginasi inkremental tahap 0–1 | Merged |
| #15 | Paginasi inkremental tahap 2 | Merged |
| #16 | Paginasi inkremental tahap 3 | Merged |
| #23 | Revert bump font (#18) | Merged |
| #24 | Pembaruan: periksa rilis repo ini | Merged |
| #25 | OpenGL di mesin Windows ber-GPU Adreno | Merged |
| #26 | Dialog berkas tidak memajukan jendela pertama | Merged |
| #27 | Export: kembali ke dokumen dan pemberitahuan | Merged |
| #28 | Ribbon: pratinjau langsung di tab Home | Terbuka saat nota ditulis |
| #29 | About: repo ini, pemiliknya, dan Claude Fable 5.2 | Terbuka saat nota ditulis |

PR #17 sampai #22 adalah pembaruan dependensi dari Dependabot: #17, #20, #21, dan #22 di-merge; #18 di-merge lalu di-revert; #19 ditutup.
