# Chess piece artwork

The graphics in `fancy/` are reused unchanged from GNOME Chess revision
`1899fcc91afbb455e2d4f60ba011c504c607973c`.

Original source: <https://github.com/GNOME/gnome-chess/tree/main/data/pieces/fancy>

Copyright © 2010 Alexey Kryukov. Each SVG is licensed under the GNU General
Public License, version 2 or (at your option) any later version. The full
license notice is preserved inside each file. This project distributes them
under GPL-3.0-or-later, as permitted by that license.

The corresponding 512×512 PNG files are unmodified renderings generated with
`rsvg-convert`. They are embedded in the application because GTK's texture
loader does not consistently decode SVG data from memory. The original SVGs
remain the preferred source form.
