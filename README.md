# Anyrun plugin for the pass password manager

## Developing with Nix

The flake included in this repo builds the package and also provides a dev shell.
To use the shell run:

```bash
$ nix develop
```

then inside that shell there is a bash alias `anyrun-pass` which will run anyrun with the just built plugin.
