<!------------------------------------------------------------------------->

[license]: https://img.shields.io/github/license/wert007/tabelle
[repository]: https://github.com/wert007/tabelle

<!------------------------------------------------------------------------->

# tabelle

## Summary

[![][license]][repository]

A simple `.csv` and `.xlsx` viewer for your terminal.

## Running & Commandline Args

You can open a file by typing `tabelle file.csv` or just start a new one by
running `tabelle`.

## Features

It supports formulas, just like any other spreadsheet program. They start
with an `=` and then contain python code. You can refer to columns and cells
by their names, both in UPPERCASE and lowercase (not mixed though!). If you
save as csv it will just save the value of the formula. To keep the formula
use the `.xlsx` format.

## Installation

You need cargo installed to install this, then just execute this command:

```bash
cargo install --git https://github.com/wert007/tabelle
```

## Contributions

This is just a small personal project for me, at the same time I feel like
there is an empty niche for terminal spreadsheet viewer. I personally add
features, when I will need them, if you want to add features of your own
feel free to open an issue or a pull request. Just make sure to run `cargo
fmt` and `cargo clippy` before opening your pull request.
