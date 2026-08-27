# Icon placeholders

Tauri requires `src-tauri/icons/` to contain at least one of:

- `32x32.png`
- `128x128.png`
- `icon.png`
- `icon.ico`

To produce real icons for release builds, drop your icon files into this
directory. Recommended source: a 1024x1024 PNG, then use
`tauri icon ./source.png` to generate the full set.

Until you do that, `pnpm tauri build` will fail. `pnpm tauri dev` will run
without a custom icon (Tauri will draw a placeholder).
