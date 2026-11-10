# RF-EPiano

![RF-EPiano instrument](package/branding/banner.png)

RF-EPiano is a 200A-style reed electric piano for RackForge. Play softly for a rounded tone or dig in to hear the reed and pickup bark. The cabinet sound and tremolo are both available from the front panel.

## Install

RF-EPiano is included in the RackForge Standard bundle. To install it separately, download `RF-EPiano.rfplugin` and choose **Install plugin** in RackForge's **Plugin Manager**. [Download RF-EPiano](https://github.com/kalexis1994/rackforge-plugin-rf-epiano/releases/latest)

## Choose a program

Open **RF-EPiano** in PLAY. Use the arrows in **Program memory** to step through the five factory sounds, or select the program name to search.

| Program | Sound |
| --- | --- |
| **200A Original** | The main 200A voicing. |
| **200A Direct** | A direct signal that leaves room for your own effects. |
| **200A Cabinet** | The tone of the modelled speaker cabinet. |
| **200A Cabinet Tremolo** | Cabinet sound with tremolo already engaged. |
| **200A Bark** | A more assertive voicing for accents and rhythmic playing. |

## Work the front panel

- **Volume** sets the piano's output.
- **Tremolo** sets how strongly the loudness pulses. Try it on held chords first.
- **Speaker** blends the direct output with the cabinet. Sweep it while playing the same phrase to hear what the speaker adds.

Save your own settings under **Presets** in PLAY. You can import and export saved presets as `.rfpreset` files.

RF-EPiano is based on the [OpenWurli DSP](https://github.com/hal0zer0/openwurli). The release also provides a source archive. [OpenWurli provenance](vendor/openwurli-dsp/UPSTREAM.md) · [RackForge SDK revision](https://github.com/kalexis1994/rackforge/tree/7a14c68aa208290587d985d606b0dbc17600ee07/crates/rackforge-plugin-sdk) · [Melange primitives revision](https://github.com/hal0zer0/melange/tree/7ecb36c7e237c66929a39f936734b8b5a63f128d)

## License

RF-EPiano is distributed under GPL-3.0-or-later. [License](LICENSE) · [Notices](package/notices/NOTICE.txt)
