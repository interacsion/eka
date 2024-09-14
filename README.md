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

For detailed usage, run `eka publish help`.

---

_No more half-measures, no more compromises, and please, no more wrappers..._

[eos]: https://github.com/ekala-project/eos-gateway
