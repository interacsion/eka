# Eka CLI

> ⚠️ Warning: Eka is in early experimental stages. Features are unstable and subject to change.

Eka is envisioned as a next-generation functional evaluation frontend for the Ekala development platform, aiming to offer a seamless, extensible, and powerful interface for managing complex software projects.

## A Reasonable Interface

CLI interactions with Nix builds (e.g. nix-shell) can halt for undecidable periods before even starting to build. In contrast, commands like `cargo build` operate on static, committable information, allowing them to begin work immediately.

To bridge this gap, Eka's working design incorporates:

1. A statically determinable interface
2. Integration with advanced build schedulers ([Eos][eos])
3. An extensible, schema-driven plugin system
4. The Atom format: Verifiable, versioned repository slices

## Usage

Currently, Eka provides the `publish` subcommand:

```
eka publish [OPTIONS] [PATH]...
```

This command implements an in-source publishing strategy for Atoms. It creates snapshots separate from the main repository history, enabling efficient, path-based versioning without a separate registry. This lays the groundwork for future decentralized resolution to a standard lock format _a la_ `eka resolve`.

For more detailed usage, run `eka help`.

_No more half-measures, no more compromises, and please, no more wrappers..._

## Provisional Road Map

Eka is still fairly early in development, however, the foundation piece, the atom format is more or less stable, in isolation. The following is an outline of the steps along the path toward a relatively stable first cut of eka:
- [x] define atom format
  - [x] implement git atom store
  - [ ] implement s3 atom store
  - [ ] ... decide on other atom storage mechanisms for 1.0
- [ ] integrate eka with atom modules
  - [x] implement the [atom](https://github.com/ekala-project/atom) module system
  - [ ] define a clean interface between the Nix module system for atom's and eka
    - [ ] implement the (pure nix) PoC manifest (with revisions) in Eka directly
- [ ] implement atom dependencies
  - [ ] implement "shallow" dependency resolution algorithm
  - [ ] implement "deep" dependency resolution using an SAT solver (resolvo crate)
- [ ] implement eka plugins
  - [ ] define cross-language plugin interface
  - [ ] allow plugins to extend the atom manifest in a principled (type-safe) manner
- [ ] implement cli subcommands
  - [x] publish subcommand (for git stores)
  - [ ] init subcommand
    - [x] git store initialization
    - [ ] user friendly initilization flow
    - [ ] init other atom stores (dependent on store implementation)
  - [ ] `list` subcommand
  - [ ] add more here as they are decided

[eos]: https://github.com/ekala-project/eos-gateway
