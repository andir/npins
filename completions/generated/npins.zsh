#compdef npins

autoload -U is-at-least

_npins() {
    typeset -A opt_args
    typeset -a _arguments_options
    local ret=1

    if is-at-least 5.2; then
        _arguments_options=(-s -S -C)
    else
        _arguments_options=(-s -C)
    fi

    local context curcontext="$curcontext" state line
    _arguments "${_arguments_options[@]}" : \
'-d+[Base folder for sources.json and the boilerplate default.nix]:FOLDER:_files -/' \
'--directory=[Base folder for sources.json and the boilerplate default.nix]:FOLDER:_files -/' \
'--lock-file=[Specifies the path to the sources.json and activates lockfile mode. In lockfile mode, no default.nix will be generated and --directory will be ignored]:LOCK_FILE:_files' \
'-v[Print debug messages]' \
'--verbose[Print debug messages]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
":: :_npins_commands" \
"*::: :->npins-completions" \
&& ret=0
    case $state in
    (npins-completions)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:npins-command-$line[1]:"
        case $line[1] in
            (init)
_arguments "${_arguments_options[@]}" : \
'--bare[Don'\''t add an initial \`nixpkgs\` entry]' \
'-v[Print debug messages]' \
'--verbose[Print debug messages]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(add)
_arguments "${_arguments_options[@]}" : \
'--name=[Add the pin with a custom name. If a pin with that name already exists, it will be overwritten]:NAME:' \
'--frozen[Add the pin as frozen, meaning that it will be ignored by \`npins update\` by default]' \
'-n[Don'\''t actually apply the changes]' \
'--dry-run[Don'\''t actually apply the changes]' \
'-v[Print debug messages]' \
'--verbose[Print debug messages]' \
'-h[Print help]' \
'--help[Print help]' \
":: :_npins__add_commands" \
"*::: :->add" \
&& ret=0

    case $state in
    (add)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:npins-add-command-$line[1]:"
        case $line[1] in
            (channel)
_arguments "${_arguments_options[@]}" : \
'--name=[Add the pin with a custom name. If a pin with that name already exists, it will be overwritten]:NAME:' \
'--frozen[Add the pin as frozen, meaning that it will be ignored by \`npins update\` by default]' \
'-v[Print debug messages]' \
'--verbose[Print debug messages]' \
'-h[Print help]' \
'--help[Print help]' \
':channel_name:' \
&& ret=0
;;
(github)
_arguments "${_arguments_options[@]}" : \
'-b+[Track a branch instead of a release]:BRANCH:' \
'--branch=[Track a branch instead of a release]:BRANCH:' \
'--at=[Use a specific commit/release instead of the latest. This may be a tag name, or a git revision when --branch is set]:tag or rev:' \
'(-b --branch --at)--upper-bound=[Bound the version resolution. For example, setting this to "2" will restrict updates to 1.X versions. Conflicts with the --branch option]:version:' \
'--release-prefix=[Optional prefix required for each release name / tag. For example, setting this to "release/" will only consider those that start with that string]:RELEASE_PREFIX:' \
'--name=[Add the pin with a custom name. If a pin with that name already exists, it will be overwritten]:NAME:' \
'(-b --branch)--pre-releases[Also track pre-releases. Conflicts with the --branch option]' \
'--submodules[Also fetch submodules]' \
'--frozen[Add the pin as frozen, meaning that it will be ignored by \`npins update\` by default]' \
'-v[Print debug messages]' \
'--verbose[Print debug messages]' \
'-h[Print help]' \
'--help[Print help]' \
':owner:' \
':repository:' \
&& ret=0
;;
(forgejo)
_arguments "${_arguments_options[@]}" : \
'-b+[Track a branch instead of a release]:BRANCH:' \
'--branch=[Track a branch instead of a release]:BRANCH:' \
'--at=[Use a specific commit/release instead of the latest. This may be a tag name, or a git revision when --branch is set]:tag or rev:' \
'(-b --branch --at)--upper-bound=[Bound the version resolution. For example, setting this to "2" will restrict updates to 1.X versions. Conflicts with the --branch option]:version:' \
'--release-prefix=[Optional prefix required for each release name / tag. For example, setting this to "release/" will only consider those that start with that string]:RELEASE_PREFIX:' \
'--name=[Add the pin with a custom name. If a pin with that name already exists, it will be overwritten]:NAME:' \
'(-b --branch)--pre-releases[Also track pre-releases. Conflicts with the --branch option]' \
'--submodules[Also fetch submodules]' \
'--frozen[Add the pin as frozen, meaning that it will be ignored by \`npins update\` by default]' \
'-v[Print debug messages]' \
'--verbose[Print debug messages]' \
'-h[Print help]' \
'--help[Print help]' \
':server:_urls' \
':owner:' \
':repository:' \
&& ret=0
;;
(gitlab)
_arguments "${_arguments_options[@]}" : \
'--server=[Use a self-hosted GitLab instance instead]:url:_urls' \
'--private-token=[Use a private token to access the repository.]:token:' \
'-b+[Track a branch instead of a release]:BRANCH:' \
'--branch=[Track a branch instead of a release]:BRANCH:' \
'--at=[Use a specific commit/release instead of the latest. This may be a tag name, or a git revision when --branch is set]:tag or rev:' \
'(-b --branch --at)--upper-bound=[Bound the version resolution. For example, setting this to "2" will restrict updates to 1.X versions. Conflicts with the --branch option]:version:' \
'--release-prefix=[Optional prefix required for each release name / tag. For example, setting this to "release/" will only consider those that start with that string]:RELEASE_PREFIX:' \
'--name=[Add the pin with a custom name. If a pin with that name already exists, it will be overwritten]:NAME:' \
'(-b --branch)--pre-releases[Also track pre-releases. Conflicts with the --branch option]' \
'--submodules[Also fetch submodules]' \
'--frozen[Add the pin as frozen, meaning that it will be ignored by \`npins update\` by default]' \
'-v[Print debug messages]' \
'--verbose[Print debug messages]' \
'-h[Print help]' \
'--help[Print help]' \
'*::repo_path -- Usually just `"owner" "repository"`, but GitLab allows arbitrary folder-like structures:' \
&& ret=0
;;
(git)
_arguments "${_arguments_options[@]}" : \
'--forge=[]:FORGE:((none\:"A generic git pin, with no further information"
auto\:"Try to determine the Forge from the given url, potentially by probing the server"
gitlab\:"A Gitlab forge, e.g. gitlab.com"
github\:"A Github forge, i.e. github.com"
forgejo\:"A Forgejo forge, e.g. forgejo.org"))' \
'-b+[Track a branch instead of a release]:BRANCH:' \
'--branch=[Track a branch instead of a release]:BRANCH:' \
'--at=[Use a specific commit/release instead of the latest. This may be a tag name, or a git revision when --branch is set]:tag or rev:' \
'(-b --branch --at)--upper-bound=[Bound the version resolution. For example, setting this to "2" will restrict updates to 1.X versions. Conflicts with the --branch option]:version:' \
'--release-prefix=[Optional prefix required for each release name / tag. For example, setting this to "release/" will only consider those that start with that string]:RELEASE_PREFIX:' \
'--name=[Add the pin with a custom name. If a pin with that name already exists, it will be overwritten]:NAME:' \
'(-b --branch)--pre-releases[Also track pre-releases. Conflicts with the --branch option]' \
'--submodules[Also fetch submodules]' \
'--frozen[Add the pin as frozen, meaning that it will be ignored by \`npins update\` by default]' \
'-v[Print debug messages]' \
'--verbose[Print debug messages]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
':url -- The git remote URL. For example <https\://github.com/andir/ate.git>:_urls' \
&& ret=0
;;
(pypi)
_arguments "${_arguments_options[@]}" : \
'--at=[Use a specific release instead of the latest]:version:' \
'(--at)--upper-bound=[Bound the version resolution. For example, setting this to "2" will restrict updates to 1.X versions. Conflicts with the --branch option]:version:' \
'--name=[Add the pin with a custom name. If a pin with that name already exists, it will be overwritten]:NAME:' \
'--frozen[Add the pin as frozen, meaning that it will be ignored by \`npins update\` by default]' \
'-v[Print debug messages]' \
'--verbose[Print debug messages]' \
'-h[Print help]' \
'--help[Print help]' \
':package_name -- Name of the package at PyPi.org:' \
&& ret=0
;;
(container)
_arguments "${_arguments_options[@]}" : \
'--name=[Add the pin with a custom name. If a pin with that name already exists, it will be overwritten]:NAME:' \
'--frozen[Add the pin as frozen, meaning that it will be ignored by \`npins update\` by default]' \
'-v[Print debug messages]' \
'--verbose[Print debug messages]' \
'-h[Print help]' \
'--help[Print help]' \
':image_name:' \
':image_tag:' \
&& ret=0
;;
(tarball)
_arguments "${_arguments_options[@]}" : \
'--name=[Add the pin with a custom name. If a pin with that name already exists, it will be overwritten]:NAME:' \
'--frozen[Add the pin as frozen, meaning that it will be ignored by \`npins update\` by default]' \
'-v[Print debug messages]' \
'--verbose[Print debug messages]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
':url -- Tarball URL:_urls' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_npins__add__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:npins-add-help-command-$line[1]:"
        case $line[1] in
            (channel)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(github)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(forgejo)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(gitlab)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(git)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(pypi)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(container)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(tarball)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(show)
_arguments "${_arguments_options[@]}" : \
'-p[Prints only pin names]' \
'--plain[Prints only pin names]' \
'-e[Invert \[NAMES\] to exclude specified pins]' \
'--exclude[Invert \[NAMES\] to exclude specified pins]' \
'-v[Print debug messages]' \
'--verbose[Print debug messages]' \
'-h[Print help]' \
'--help[Print help]' \
'*::names -- Names of the pins to show:' \
&& ret=0
;;
(update)
_arguments "${_arguments_options[@]}" : \
'--max-concurrent-downloads=[Maximum number of simultaneous downloads]:MAX_CONCURRENT_DOWNLOADS:' \
'(-f --full)-p[Don'\''t update versions, only re-fetch hashes]' \
'(-f --full)--partial[Don'\''t update versions, only re-fetch hashes]' \
'(-p --partial)-f[Re-fetch hashes even if the version hasn'\''t changed. Useful to make sure the derivations are in the Nix store]' \
'(-p --partial)--full[Re-fetch hashes even if the version hasn'\''t changed. Useful to make sure the derivations are in the Nix store]' \
'-n[Print the diff, but don'\''t write back the changes]' \
'--dry-run[Print the diff, but don'\''t write back the changes]' \
'--frozen[Allow updating frozen pins, which would otherwise be ignored]' \
'-v[Print debug messages]' \
'--verbose[Print debug messages]' \
'-h[Print help]' \
'--help[Print help]' \
'*::names -- Updates only the specified pins:' \
&& ret=0
;;
(verify)
_arguments "${_arguments_options[@]}" : \
'--max-concurrent-downloads=[Maximum number of simultaneous downloads]:MAX_CONCURRENT_DOWNLOADS:' \
'-v[Print debug messages]' \
'--verbose[Print debug messages]' \
'-h[Print help]' \
'--help[Print help]' \
'*::names -- Verifies only the specified pins:' \
&& ret=0
;;
(upgrade)
_arguments "${_arguments_options[@]}" : \
'-v[Print debug messages]' \
'--verbose[Print debug messages]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(remove)
_arguments "${_arguments_options[@]}" : \
'-v[Print debug messages]' \
'--verbose[Print debug messages]' \
'-h[Print help]' \
'--help[Print help]' \
'*::names:' \
&& ret=0
;;
(import-niv)
_arguments "${_arguments_options[@]}" : \
'-n+[Only import one entry from Niv]:NAME:' \
'--name=[Only import one entry from Niv]:NAME:' \
'-v[Print debug messages]' \
'--verbose[Print debug messages]' \
'-h[Print help]' \
'--help[Print help]' \
'::path:_files' \
&& ret=0
;;
(import-flake)
_arguments "${_arguments_options[@]}" : \
'-n+[Only import one entry from the flake]:NAME:' \
'--name=[Only import one entry from the flake]:NAME:' \
'-v[Print debug messages]' \
'--verbose[Print debug messages]' \
'-h[Print help]' \
'--help[Print help]' \
'::path:_files' \
&& ret=0
;;
(freeze)
_arguments "${_arguments_options[@]}" : \
'-v[Print debug messages]' \
'--verbose[Print debug messages]' \
'-h[Print help]' \
'--help[Print help]' \
'*::names -- Names of the pin(s):' \
&& ret=0
;;
(unfreeze)
_arguments "${_arguments_options[@]}" : \
'-v[Print debug messages]' \
'--verbose[Print debug messages]' \
'-h[Print help]' \
'--help[Print help]' \
'*::names -- Names of the pin(s):' \
&& ret=0
;;
(get-path)
_arguments "${_arguments_options[@]}" : \
'-v[Print debug messages]' \
'--verbose[Print debug messages]' \
'-h[Print help]' \
'--help[Print help]' \
':name -- Name of the pin:' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_npins__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:npins-help-command-$line[1]:"
        case $line[1] in
            (init)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(add)
_arguments "${_arguments_options[@]}" : \
":: :_npins__help__add_commands" \
"*::: :->add" \
&& ret=0

    case $state in
    (add)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:npins-help-add-command-$line[1]:"
        case $line[1] in
            (channel)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(github)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(forgejo)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(gitlab)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(git)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(pypi)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(container)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(tarball)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(show)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(update)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(verify)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(upgrade)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(remove)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(import-niv)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(import-flake)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(freeze)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(unfreeze)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(get-path)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
}

(( $+functions[_npins_commands] )) ||
_npins_commands() {
    local commands; commands=(
'init:Intializes the npins directory. Running this multiple times will restore/upgrade the \`default.nix\` and never touch your sources.json' \
'add:Adds a new pin entry' \
'show:Lists the current pin entries' \
'update:Updates all or the given pins to the latest version' \
'verify:Verifies that all or the given pins still have correct hashes. This is like \`update --partial --dry-run\` and then checking that the diff is empty' \
'upgrade:Upgrade the sources.json and default.nix to the latest format version. This may occasionally break Nix evaluation!' \
'remove:Removes one pin entry' \
'import-niv:Try to import entries from Niv' \
'import-flake:Try to import entries from flake.lock' \
'freeze:Freeze a pin entry' \
'unfreeze:Thaw a pin entry' \
'get-path:Evaluates the store path to a pin, fetching it if necessary. Don'\''t forget to add a GC root' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'npins commands' commands "$@"
}
(( $+functions[_npins__add_commands] )) ||
_npins__add_commands() {
    local commands; commands=(
'channel:Track a Nix channel' \
'github:Track a GitHub repository' \
'forgejo:Track a Forgejo repository' \
'gitlab:Track a GitLab repository' \
'git:Track a git repository' \
'pypi:Track a package on PyPi' \
'container:Track an OCI container' \
'tarball:Track a tarball' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'npins add commands' commands "$@"
}
(( $+functions[_npins__add__channel_commands] )) ||
_npins__add__channel_commands() {
    local commands; commands=()
    _describe -t commands 'npins add channel commands' commands "$@"
}
(( $+functions[_npins__add__container_commands] )) ||
_npins__add__container_commands() {
    local commands; commands=()
    _describe -t commands 'npins add container commands' commands "$@"
}
(( $+functions[_npins__add__forgejo_commands] )) ||
_npins__add__forgejo_commands() {
    local commands; commands=()
    _describe -t commands 'npins add forgejo commands' commands "$@"
}
(( $+functions[_npins__add__git_commands] )) ||
_npins__add__git_commands() {
    local commands; commands=()
    _describe -t commands 'npins add git commands' commands "$@"
}
(( $+functions[_npins__add__github_commands] )) ||
_npins__add__github_commands() {
    local commands; commands=()
    _describe -t commands 'npins add github commands' commands "$@"
}
(( $+functions[_npins__add__gitlab_commands] )) ||
_npins__add__gitlab_commands() {
    local commands; commands=()
    _describe -t commands 'npins add gitlab commands' commands "$@"
}
(( $+functions[_npins__add__help_commands] )) ||
_npins__add__help_commands() {
    local commands; commands=(
'channel:Track a Nix channel' \
'github:Track a GitHub repository' \
'forgejo:Track a Forgejo repository' \
'gitlab:Track a GitLab repository' \
'git:Track a git repository' \
'pypi:Track a package on PyPi' \
'container:Track an OCI container' \
'tarball:Track a tarball' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'npins add help commands' commands "$@"
}
(( $+functions[_npins__add__help__channel_commands] )) ||
_npins__add__help__channel_commands() {
    local commands; commands=()
    _describe -t commands 'npins add help channel commands' commands "$@"
}
(( $+functions[_npins__add__help__container_commands] )) ||
_npins__add__help__container_commands() {
    local commands; commands=()
    _describe -t commands 'npins add help container commands' commands "$@"
}
(( $+functions[_npins__add__help__forgejo_commands] )) ||
_npins__add__help__forgejo_commands() {
    local commands; commands=()
    _describe -t commands 'npins add help forgejo commands' commands "$@"
}
(( $+functions[_npins__add__help__git_commands] )) ||
_npins__add__help__git_commands() {
    local commands; commands=()
    _describe -t commands 'npins add help git commands' commands "$@"
}
(( $+functions[_npins__add__help__github_commands] )) ||
_npins__add__help__github_commands() {
    local commands; commands=()
    _describe -t commands 'npins add help github commands' commands "$@"
}
(( $+functions[_npins__add__help__gitlab_commands] )) ||
_npins__add__help__gitlab_commands() {
    local commands; commands=()
    _describe -t commands 'npins add help gitlab commands' commands "$@"
}
(( $+functions[_npins__add__help__help_commands] )) ||
_npins__add__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'npins add help help commands' commands "$@"
}
(( $+functions[_npins__add__help__pypi_commands] )) ||
_npins__add__help__pypi_commands() {
    local commands; commands=()
    _describe -t commands 'npins add help pypi commands' commands "$@"
}
(( $+functions[_npins__add__help__tarball_commands] )) ||
_npins__add__help__tarball_commands() {
    local commands; commands=()
    _describe -t commands 'npins add help tarball commands' commands "$@"
}
(( $+functions[_npins__add__pypi_commands] )) ||
_npins__add__pypi_commands() {
    local commands; commands=()
    _describe -t commands 'npins add pypi commands' commands "$@"
}
(( $+functions[_npins__add__tarball_commands] )) ||
_npins__add__tarball_commands() {
    local commands; commands=()
    _describe -t commands 'npins add tarball commands' commands "$@"
}
(( $+functions[_npins__freeze_commands] )) ||
_npins__freeze_commands() {
    local commands; commands=()
    _describe -t commands 'npins freeze commands' commands "$@"
}
(( $+functions[_npins__get-path_commands] )) ||
_npins__get-path_commands() {
    local commands; commands=()
    _describe -t commands 'npins get-path commands' commands "$@"
}
(( $+functions[_npins__help_commands] )) ||
_npins__help_commands() {
    local commands; commands=(
'init:Intializes the npins directory. Running this multiple times will restore/upgrade the \`default.nix\` and never touch your sources.json' \
'add:Adds a new pin entry' \
'show:Lists the current pin entries' \
'update:Updates all or the given pins to the latest version' \
'verify:Verifies that all or the given pins still have correct hashes. This is like \`update --partial --dry-run\` and then checking that the diff is empty' \
'upgrade:Upgrade the sources.json and default.nix to the latest format version. This may occasionally break Nix evaluation!' \
'remove:Removes one pin entry' \
'import-niv:Try to import entries from Niv' \
'import-flake:Try to import entries from flake.lock' \
'freeze:Freeze a pin entry' \
'unfreeze:Thaw a pin entry' \
'get-path:Evaluates the store path to a pin, fetching it if necessary. Don'\''t forget to add a GC root' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'npins help commands' commands "$@"
}
(( $+functions[_npins__help__add_commands] )) ||
_npins__help__add_commands() {
    local commands; commands=(
'channel:Track a Nix channel' \
'github:Track a GitHub repository' \
'forgejo:Track a Forgejo repository' \
'gitlab:Track a GitLab repository' \
'git:Track a git repository' \
'pypi:Track a package on PyPi' \
'container:Track an OCI container' \
'tarball:Track a tarball' \
    )
    _describe -t commands 'npins help add commands' commands "$@"
}
(( $+functions[_npins__help__add__channel_commands] )) ||
_npins__help__add__channel_commands() {
    local commands; commands=()
    _describe -t commands 'npins help add channel commands' commands "$@"
}
(( $+functions[_npins__help__add__container_commands] )) ||
_npins__help__add__container_commands() {
    local commands; commands=()
    _describe -t commands 'npins help add container commands' commands "$@"
}
(( $+functions[_npins__help__add__forgejo_commands] )) ||
_npins__help__add__forgejo_commands() {
    local commands; commands=()
    _describe -t commands 'npins help add forgejo commands' commands "$@"
}
(( $+functions[_npins__help__add__git_commands] )) ||
_npins__help__add__git_commands() {
    local commands; commands=()
    _describe -t commands 'npins help add git commands' commands "$@"
}
(( $+functions[_npins__help__add__github_commands] )) ||
_npins__help__add__github_commands() {
    local commands; commands=()
    _describe -t commands 'npins help add github commands' commands "$@"
}
(( $+functions[_npins__help__add__gitlab_commands] )) ||
_npins__help__add__gitlab_commands() {
    local commands; commands=()
    _describe -t commands 'npins help add gitlab commands' commands "$@"
}
(( $+functions[_npins__help__add__pypi_commands] )) ||
_npins__help__add__pypi_commands() {
    local commands; commands=()
    _describe -t commands 'npins help add pypi commands' commands "$@"
}
(( $+functions[_npins__help__add__tarball_commands] )) ||
_npins__help__add__tarball_commands() {
    local commands; commands=()
    _describe -t commands 'npins help add tarball commands' commands "$@"
}
(( $+functions[_npins__help__freeze_commands] )) ||
_npins__help__freeze_commands() {
    local commands; commands=()
    _describe -t commands 'npins help freeze commands' commands "$@"
}
(( $+functions[_npins__help__get-path_commands] )) ||
_npins__help__get-path_commands() {
    local commands; commands=()
    _describe -t commands 'npins help get-path commands' commands "$@"
}
(( $+functions[_npins__help__help_commands] )) ||
_npins__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'npins help help commands' commands "$@"
}
(( $+functions[_npins__help__import-flake_commands] )) ||
_npins__help__import-flake_commands() {
    local commands; commands=()
    _describe -t commands 'npins help import-flake commands' commands "$@"
}
(( $+functions[_npins__help__import-niv_commands] )) ||
_npins__help__import-niv_commands() {
    local commands; commands=()
    _describe -t commands 'npins help import-niv commands' commands "$@"
}
(( $+functions[_npins__help__init_commands] )) ||
_npins__help__init_commands() {
    local commands; commands=()
    _describe -t commands 'npins help init commands' commands "$@"
}
(( $+functions[_npins__help__remove_commands] )) ||
_npins__help__remove_commands() {
    local commands; commands=()
    _describe -t commands 'npins help remove commands' commands "$@"
}
(( $+functions[_npins__help__show_commands] )) ||
_npins__help__show_commands() {
    local commands; commands=()
    _describe -t commands 'npins help show commands' commands "$@"
}
(( $+functions[_npins__help__unfreeze_commands] )) ||
_npins__help__unfreeze_commands() {
    local commands; commands=()
    _describe -t commands 'npins help unfreeze commands' commands "$@"
}
(( $+functions[_npins__help__update_commands] )) ||
_npins__help__update_commands() {
    local commands; commands=()
    _describe -t commands 'npins help update commands' commands "$@"
}
(( $+functions[_npins__help__upgrade_commands] )) ||
_npins__help__upgrade_commands() {
    local commands; commands=()
    _describe -t commands 'npins help upgrade commands' commands "$@"
}
(( $+functions[_npins__help__verify_commands] )) ||
_npins__help__verify_commands() {
    local commands; commands=()
    _describe -t commands 'npins help verify commands' commands "$@"
}
(( $+functions[_npins__import-flake_commands] )) ||
_npins__import-flake_commands() {
    local commands; commands=()
    _describe -t commands 'npins import-flake commands' commands "$@"
}
(( $+functions[_npins__import-niv_commands] )) ||
_npins__import-niv_commands() {
    local commands; commands=()
    _describe -t commands 'npins import-niv commands' commands "$@"
}
(( $+functions[_npins__init_commands] )) ||
_npins__init_commands() {
    local commands; commands=()
    _describe -t commands 'npins init commands' commands "$@"
}
(( $+functions[_npins__remove_commands] )) ||
_npins__remove_commands() {
    local commands; commands=()
    _describe -t commands 'npins remove commands' commands "$@"
}
(( $+functions[_npins__show_commands] )) ||
_npins__show_commands() {
    local commands; commands=()
    _describe -t commands 'npins show commands' commands "$@"
}
(( $+functions[_npins__unfreeze_commands] )) ||
_npins__unfreeze_commands() {
    local commands; commands=()
    _describe -t commands 'npins unfreeze commands' commands "$@"
}
(( $+functions[_npins__update_commands] )) ||
_npins__update_commands() {
    local commands; commands=()
    _describe -t commands 'npins update commands' commands "$@"
}
(( $+functions[_npins__upgrade_commands] )) ||
_npins__upgrade_commands() {
    local commands; commands=()
    _describe -t commands 'npins upgrade commands' commands "$@"
}
(( $+functions[_npins__verify_commands] )) ||
_npins__verify_commands() {
    local commands; commands=()
    _describe -t commands 'npins verify commands' commands "$@"
}

if [ "$funcstack[1]" = "_npins" ]; then
    _npins "$@"
else
    compdef _npins npins
fi
