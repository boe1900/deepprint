# Third-Party Notices

DeepPrint Studio builds on excellent open source projects. This document records the major third-party projects that are especially visible in the product and deployment workflow.

## hanxi/cups

- Project: [hanxi/cups-web](https://github.com/hanxi/cups-web)
- Docker Hub: [hanxi/cups-web](https://hub.docker.com/r/hanxi/cups-web)
- Docker image used by this project: `hanxi/cups:latest`
- License: [MIT License](https://github.com/hanxi/cups-web/blob/master/LICENSE)

DeepPrint Studio uses the `hanxi/cups` Docker image in `docker-compose.yml` as the default CUPS development and testing service. The image includes CUPS and helpful printer drivers such as `printer-driver-cups-pdf`, which lets users validate the real CUPS / IPP print path without a physical printer.

We are grateful to hanxi/cups-web for making CUPS-based printer sharing easy to run and easy to test.

Legal and release notes:

- DeepPrint Studio references the public Docker image; it does not vendor or modify hanxi/cups-web source code.
- MIT-licensed source code is compatible with DeepPrint Studio's Apache-2.0 license when copyright and license notices are preserved.
- If a future DeepPrint Studio release redistributes a derived CUPS image instead of referencing `hanxi/cups`, that image should preserve relevant license notices for hanxi/cups-web and all packages bundled in the image.
- `hanxi/cups` is a third-party image, not an official DeepPrint Studio artifact. For production deployments, consider pinning a specific image tag or digest instead of using `latest`.

## Typst

- Project: [Typst](https://typst.app/)
- Compiler source: [typst/typst](https://github.com/typst/typst)
- License: [Apache License 2.0](https://github.com/typst/typst/blob/main/LICENSE)

DeepPrint Studio uses the Typst ecosystem to render templates and preview PDFs. Typst provides a modern markup-based typesetting workflow that is much easier to operate in business template scenarios than hand-written PDF generation.

We are grateful to the Typst project for making high-quality document generation approachable and scriptable.

Legal and release notes:

- Typst's Apache-2.0 license is compatible with DeepPrint Studio's Apache-2.0 license.
- DeepPrint Studio is not affiliated with, endorsed by, or sponsored by Typst GmbH.
- User-installed Typst packages, custom fonts, images, and template assets may have their own licenses. Users should verify those licenses before commercial use or redistribution.

## CUPS

- Project: [OpenPrinting CUPS](https://github.com/OpenPrinting/cups)
- Website: [cups.org](https://www.cups.org/)

CUPS is the standard printing system behind DeepPrint Studio's printer discovery, printer capability reading, and print job submission flow.

DeepPrint Studio talks to CUPS through IPP/CUPS interfaces and does not claim ownership of CUPS, CUPS logos, printer drivers, or vendor-specific printer capabilities.
