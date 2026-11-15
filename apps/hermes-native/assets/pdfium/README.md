# PDFium runtime

The Windows x64 PDF preview runtime is from the immutable `chromium/8125` release of `bblanchon/pdfium-binaries`. The application binds the matching PDFium 8125 API through `pdfium-render`. Keep the release license and all third-party license files with the DLL. `PROVENANCE.json` records the release URL and locally verified archive and DLL SHA-256 values.

This binary is a runtime dependency for PDF previews. Other platforms need their matching PDFium 8125 shared library before PDF preview is enabled.
