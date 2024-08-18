> Coming Soon

_No more half-measures, no more compromises, and please, no more wrappers..._

# Ekala CLI

Ekala is a next-generation functional evaluation frontend and development platform. It offers a seamless, extensible, and powerful interface for managing complex software projects.

Our goal is to streamline complex workflows across all types of projects, not just those using functional programming. Ekala leverages functional paradigms in its own architecture to ensure reproducibility and robust design. However, its plugin system allows for managing any kind of project, adapting to diverse needs and development styles. This approach combines the benefits of functional design with the flexibility to work with any programming paradigm or project structure.

## Core Concepts

### Functional Evaluation Engine

At its core, Ekala serves as an entrypoint for purely functional configuration generation, working with various functional languages and configuration formats.

### Reification Backends

Reification backends transform functional evaluations into concrete artifacts (e.g., Nix or Guix builds). These backends communicate with an abstracted API (Eos) that handles build and evaluation concerns such as scheduling. This architecture:

1. Decouples the CLI from complex build processes
2. Allows for optimized communication between frontend (CLI) and backend
3. Provides a clear scope for the CLI's responsibilities
4. Enables future refinement of the build and evaluation systems independently

In essence, reification backends in the CLI act as a connection layer, bridging the gap between user commands and the actual builder backend scheduler via the Eos API. This design ensures a clean separation of concerns and allows for flexibility in backend implementations.

### Two-Tier Extension System: Atoms and Plugins

Ekala employs a sophisticated extension system:

- **Atoms**: High-level, declarative units managed through the CLI, providing domain-specific features with a consistent interface.
- **Plugins**: A language-agnostic plugin interface (using the extism framework) that underpins atoms, allowing for flexible implementation and manifest schema extension.

Atoms act as API entrypoints, activating and calling into underlying plugins. This approach combines a clean user interface with powerful, flexible implementation capabilities.

### Cross-Language Configuration Transformation

Ekala facilitates passing configuration or generated code between different functional languages (e.g., Nickel to Nix), with well-defined schemas for validation.

## Design Philosophy

1. **Simplicity and Conciseness**: Intuitive CLI with focus on essential commands.
2. **Extensibility**: Powerful extensions through the atom/plugin system without core clutter.
3. **Clear Boundaries**: Distinct separation between core, reification backends, and atom-provided features.
4. **One Clear Way**: Generally one obvious way to accomplish each task.
5. **Declarative Management**: Atoms (and their underlying plugins) are managed declaratively.
6. **Comprehensive Coverage**: Addresses needs of various expert groups while maintaining simplicity.

## Target Domains

Ekala caters to a wide range of expert groups:

1. Package Managers
2. DevOps Engineers
3. Cloud Architects
4. Site Reliability Engineers
5. Software Architects
6. IDE/Tool Developers
7. Developer Advocates & Technical Writers

## Key Features

- Functional expression evaluation and REPL
- Declarative atom management
- Lock file handling for reproducibility
- Schema validation and extension
- Cross-language configuration transformation
- Reification backends for building and running targets
- Extensible command structure through atoms and plugins

## CLI Structure

1. Core Commands: Essential functionality for evaluation, atom management, and system-wide operations.
2. Reification Backend Commands: Low-level commands for building and running targets.
3. Atom-Provided Commands: Extensible, domain-specific commands provided by atoms (implemented by plugins).

## Future Development

Ekala is designed for future expansion, potentially including:

- Additional reification backends
- Enhanced cross-language transformation capabilities
- Advanced visualization and analysis tools
- Deeper integration with cloud platforms and CI/CD systems
- Improved monitoring and observability features

## UI First Draft

This is by no means final, and needs some critical feedback, especially for the core and backend commands. But in order to give some idea of where we are heading:

```
## Core Eka Commands

eka
  eval <expression>       Evaluate a functional expression
  repl                    Start an interactive REPL session

  atom
    add <atom>            Add an atom to the project
    remove <atom>         Remove an atom from the project
    update [<atom>]       Update all atoms or a specific atom
    list                  List all atoms in the current project

  lock
    sync                  Synchronize the atom lock file with current atom versions
    verify                Verify the integrity of the atom lock file

  schema
    validate <file>       Validate a file against the Eka schema
    show [<atom>]         Show schema for all or specific atoms

  help [<command>]        Display help information for Eka or a specific command
  version                 Show version information

## Reification Backend Commands

eka
  build <target>          Build a target using the configured backend
  run <target>            Run a built target

## Atom-Provided (plugin) Commands (examples)

eka <atom> <command>      Execute an atom-specific command

# Package Management
eka pm
  resolve
    deps                  Resolve non-atom dependencies (e.g., npm, pip)
    legacy <expr>         Resolve dependencies for legacy Nix expressions
    system                Resolve system-level dependencies
  graph <package>         Display dependency graph for a package or project

# DevOps and Build Automation
eka devops
  env
    create <name>         Create a normalized environment
    enter <name>          Enter a normalized environment
  config
    version <file>        Version a configuration file
    diff <v1> <v2>        Show differences between configurations

# Cloud Deployment
eka cloud
  deploy
    plan <config>         Create a deployment plan
    apply <plan>          Apply a deployment plan
    rollback              Rollback to the previous deployment
  recipe
    list                  List available deployment recipes
    create <name>         Create a new deployment recipe

# Site Reliability Engineering
eka sre
  monitor
    start                 Start monitoring based on current config
    status                Check monitoring status

# Software Architecture
eka architect
  design
    create <name>         Create a new system design
    visualize <design>    Generate a visual representation of a design
  analyze <stack>         Analyze a software stack for improvements

# IDE Integration
eka ide
  integrate <ide>         Set up IDE integration
  sync                    Synchronize project environment with IDE

# Documentation and Learning
eka learn
  tutorial
    list                  List available tutorials
    start <name>          Start an interactive tutorial
  doc
    generate <topic>      Generate dynamic documentation
    search <query>        Search documentation
```
