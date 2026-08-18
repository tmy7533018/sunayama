<p align="right"><b>English</b> | <a href="README.md">日本語</a></p>

<h1 align="center">sunayama</h1>

<p align="center"><i>Playing with sand in your terminal</i></p>

<table>
  <tr>
    <td align="center" width="50%"><img src="docs/opening.gif" alt="startup" /><br/><sub>On startup</sub></td>
    <td align="center" width="50%"><img src="docs/dribble.gif" alt="a few grains" /><br/><sub>Sand falls at random</sub></td>
  </tr>
  <tr>
    <td align="center" width="50%"><img src="docs/space.gif" alt="holding Space" /><br/><sub><code>Space</code> drops sand</sub></td>
    <td align="center" width="50%"><img src="docs/collapse.gif" alt="collapsing" /><br/><sub>The pile collapses</sub></td>
  </tr>
</table>

## Install

```sh
cargo install --git https://github.com/tmy7533018/sunayama
```

Nix:

```sh
nix run github:tmy7533018/sunayama
```

## Features

- ASCII art sand pile
- Timer

## Usage

```sh
sunayama                # the endless pile
sunayama --timer <dur>  # timer mode (90s / 25m / 1h)
```

- In the default mode sand piles up at random. `Space` drops more sand.
- In timer mode the sand fills over the period you give it.
- `q` / `Esc` / `Ctrl-C` quits.

To change the colours, edit `~/.config/sunayama/config`.

## License

MIT
