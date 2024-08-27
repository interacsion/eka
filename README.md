> Coming Soon

_No more half-measures, no more compromises, and please, no more wrappers..._

# Eka CLI

Eka is a next-generation functional evaluation frontend for the Ekala development platform. It offers a seamless, extensible, and powerful interface for managing complex software projects.

Our goal is to streamline complex workflows across all types of projects, not just those using functional programming. Eka leverages functional paradigms in its own architecture to ensure reproducibility and robust design. However, its plugin system allows for managing any kind of project, adapting to diverse needs and development styles. This approach combines the benefits of functional design with the flexibility to work with any programming paradigm or project structure.

## UI Draft

This is by no means final, and needs some critical feedback, especially for the core and backend commands. But in order to give some idea of where we are heading:

```
## Core Eka Commands

eka
  eval <expression>       Evaluate a functional expression
  repl                    Start an interactive REPL session

  schema
    set <key> <value>     Set a value in the manifest, return a nice error if non-sensical (against the current type)
    see <key>             Show the valid schema for this type, starting at the given key into it
    validate              Validate the Eka schema for this manifest

  help [<command>]        Display help information for Eka or a specific command
  version                 Show version information

## Version Resolution Commands

eka
  deps
    add [<dep>]           Add a dependency to the project
    remove [<dep>]        Remove a dependency from the project
    update [<dep>]        Update specific dependencies
    list                  List all dependencies in the current project

  resolve
    sync                  Synchronize the dependency lock file after manual updates
    verify                Verify the integrity of the lock file
    update                Resolve all dependencies to their latest in-bounds version
    graph ?<dep>          Display dependency graph

## Reification Backend Commands

eka
  apply [<target>]        Reify a target (build, generate, etc)

  recipe
    verify [<drv>]        Ensure the integrity of the low-level recipe (i.e. derivation files)
    link [<drv|target>]   Which higher-level targets of eka are responsible for these drv or vice-versa


## Plugin Commands (examples)

eka <plugin> <command>    Execute a plugin-specific command

pkg
  search <pkg?>           Fuzzy search for information on the given package
  compile <fmt> <pkg>     Compile a package to the specified distributable format, e.g. deb, rpm, etc
  contain [<pkg>]         Build an OCI image of the package
  isolate [<pkg>]         Create a namespaced runtime for a package from a recipe

# DevOps and Build Automation
ops
  env
    create <name>         Create a normalized environment
    enter <name>          Enter a normalized environment

  chart
    diff <v1> <v2>        Show differences between deployment configurations
    validate <chart>      Ensure your deployment recipe is sensible

  compose <env> <chart>   Deploy to the specified environment

  plan <env> <chart>      Diff of the current deployment against the requested chart

  recipe
    list                  List available deployment recipes
    create <name>         Create a new deployment recipe

# Site Reliability Engineering
eka sre
  monitor
    query <?>     Request specific information from the monitoring server

    dash
      add <spec>  Add a new dashboard from the given spec
      rm <spec>   Remove a dashboard configuration
      mod <spec>  Edit in a text-editor the given dash spec

  alert
    add <env> <spec>    Add a new alert to the given environmentt from the given declaration
    rm  <env> <spec>    Removes an alert (assuming authorization)
    mod <env> <spec>    Modify the current config in a text-editor

  respond
    resolve             Open a text prompt and write a resolution
    ack                 Acknowledge an alert
    snooze <time>       Silence an alert for the given time

  status                High-level system health

eka test
  run <object>          Run the tests for the given reifiable object (e.g. package, deployment, config, etc)
  bench <name>          Run the given benchmark

# Software Architecture
eka arc

  draft <name>            Draft an architectural decision record
  analyze <stack>         Analyze a software stack for improvements
  visualize <object>      Visualize the given object (dependency tree, call-statck, etc)

# IDE Integration
eka ide
  integrate <ide>         Set up IDE integration
  sync                    Synchronize project environment with IDE

# Documentation
eka doc
  generate <topic>        Generate dynamic documentation
  search <query>          Search documentation

  tutorial
    list                  List available tutorials
    start <name>          Start an interactive tutorial
```

## Core Concepts

### Functional Evaluation Engine

At its core, Eka serves as an entrypoint for purely functional configuration generation, working with various functional languages and configuration formats.

### Reification Backends

Reification backends transform functional evaluations into concrete artifacts (e.g., Nix or Guix builds). These backends communicate with an abstracted API (Eos) that handles build and evaluation concerns such as scheduling. This architecture:

1. Decouples the CLI from complex build processes
2. Allows for optimized communication between frontend (CLI) and backend
3. Provides a clear scope for the CLI's responsibilities
4. Enables future refinement of the build and evaluation systems independently

In essence, reification backends in the CLI act as a connection layer, bridging the gap between user commands and the actual builder backend scheduler via the Eos API. This design ensures a clean separation of concerns and allows for flexibility in backend implementations.

### Two-Tier Extension System: Schema and Plugins

Eka employs a sophisticated extension system:

- **Schema Extension**: Each plugin defines extensions to the core manifest format, defined by type. Only plugins can define new types.
- **Plugins**: A language-agnostic plugin interface (using the extism framework) that declares these schemata extensions, allowing for flexible implementation of the manifest schema extension.

The manifest acts as an entrypoint, activating and calling into underlying plugins. This approach combines a clean user interface with powerful, flexible implementation capabilities.

### Cross-Language Configuration Transformation

Eka facilitates passing configuration or generated code between different functional languages (e.g., Nickel to Nix), with well-defined schemas for validation.

## Design Philosophy

1. **Simplicity and Conciseness**: Intuitive CLI with focus on essential commands.
2. **Extensibility**: Powerful extensions through the plugin system without core clutter.
3. **Clear Boundaries**: Distinct separation between core, reification backends, and plugin-provided features.
4. **One Clear Way**: Generally one obvious way to accomplish each task.
5. **Declarative Management**: The manifest defines an entry into a fundamentally declarative platform to manage software projects.
6. **Comprehensive Coverage**: Addresses needs of various expert groups through extensions, while maintaining simplicity in the core.

## Target Domains

Eka seeks to appeal to a wide range of expert groups through its extension mechanisms

1. Software Developers
2. Package Managers
3. DevOps Engineers
4. Cloud Architects
5. Site Reliability Engineers
6. Software Architects
7. IDE/Tool Developers
8. Documentation Writers & Engineers

## Future Development

Eka is designed for future expansion, potentially including:

- Additional reification backends
- Enhanced cross-language transformation capabilities
- Advanced visualization and analysis tools
- Deeper integration with cloud platforms and CI/CD systems
- Improved monitoring and observability features
