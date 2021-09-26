# npins

Nix dependency pinning.
## npins help
```console
$ npins help
npins 0.1.0

USAGE:
    npins [folder] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <folder>    Base folder for npins.json and the boilerplate default.nix [env: NPINS_FOLDER=]  [default: npins]

SUBCOMMANDS:
    add       Adds a new pin entry
    help      Prints this message or the help of the given subcommand(s)
    init      Intializes the npins directory. Running this multiple times will restore/upgrade the `default.nix` and
              never touch your pins.json
    remove    Removes one pin entry
    show      Lists the current pin entries
    update    Updates all or the given pin to the latest version
```

## npins help init
```console
$ npins help init
npins-init 0.1.0
Intializes the npins directory. Running this multiple times will restore/upgrade the `default.nix` and never touch your
pins.json

USAGE:
    npins init

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
```

## npins help add
```console
$ npins help add
npins-add 0.1.0
Adds a new pin entry

USAGE:
    npins add [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -n, --name <name>    

SUBCOMMANDS:
    git               
    github            
    github-release    
    help              Prints this message or the help of the given subcommand(s)
```

## npins help update
```console
$ npins help update
npins-update 0.1.0
Updates all or the given pin to the latest version

USAGE:
    npins update [name]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <name>    
```

## npins help remove
```console
$ npins help remove
npins-remove 0.1.0
Removes one pin entry

USAGE:
    npins remove <name>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <name>    
```


