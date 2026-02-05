# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_npins_global_optspecs
	string join \n d/directory= lock-file= v/verbose h/help V/version
end

function __fish_npins_needs_command
	# Figure out if the current invocation already has a command.
	set -l cmd (commandline -opc)
	set -e cmd[1]
	argparse -s (__fish_npins_global_optspecs) -- $cmd 2>/dev/null
	or return
	if set -q argv[1]
		# Also print the command, so this can be used to figure out what it is.
		echo $argv[1]
		return 1
	end
	return 0
end

function __fish_npins_using_subcommand
	set -l cmd (__fish_npins_needs_command)
	test -z "$cmd"
	and return 1
	contains -- $cmd[1] $argv
end

complete -c npins -n "__fish_npins_needs_command" -s d -l directory -d 'Base folder for sources.json and the boilerplate default.nix' -r -f -a "(__fish_complete_directories)"
complete -c npins -n "__fish_npins_needs_command" -l lock-file -d 'Specifies the path to the sources.json and activates lockfile mode. In lockfile mode, no default.nix will be generated and --directory will be ignored' -r -F
complete -c npins -n "__fish_npins_needs_command" -s v -l verbose -d 'Print debug messages'
complete -c npins -n "__fish_npins_needs_command" -s h -l help -d 'Print help'
complete -c npins -n "__fish_npins_needs_command" -s V -l version -d 'Print version'
complete -c npins -n "__fish_npins_needs_command" -f -a "init" -d 'Intializes the npins directory. Running this multiple times will restore/upgrade the `default.nix` and never touch your sources.json'
complete -c npins -n "__fish_npins_needs_command" -f -a "add" -d 'Adds a new pin entry'
complete -c npins -n "__fish_npins_needs_command" -f -a "show" -d 'Lists the current pin entries'
complete -c npins -n "__fish_npins_needs_command" -f -a "update" -d 'Updates all or the given pins to the latest version'
complete -c npins -n "__fish_npins_needs_command" -f -a "verify" -d 'Verifies that all or the given pins still have correct hashes. This is like `update --partial --dry-run` and then checking that the diff is empty'
complete -c npins -n "__fish_npins_needs_command" -f -a "upgrade" -d 'Upgrade the sources.json and default.nix to the latest format version. This may occasionally break Nix evaluation!'
complete -c npins -n "__fish_npins_needs_command" -f -a "remove" -d 'Removes one pin entry'
complete -c npins -n "__fish_npins_needs_command" -f -a "import-niv" -d 'Try to import entries from Niv'
complete -c npins -n "__fish_npins_needs_command" -f -a "import-flake" -d 'Try to import entries from flake.lock'
complete -c npins -n "__fish_npins_needs_command" -f -a "freeze" -d 'Freeze a pin entry'
complete -c npins -n "__fish_npins_needs_command" -f -a "unfreeze" -d 'Thaw a pin entry'
complete -c npins -n "__fish_npins_needs_command" -f -a "get-path" -d 'Evaluates the store path to a pin, fetching it if necessary. Don\'t forget to add a GC root'
complete -c npins -n "__fish_npins_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c npins -n "__fish_npins_using_subcommand init" -l bare -d 'Don\'t add an initial `nixpkgs` entry'
complete -c npins -n "__fish_npins_using_subcommand init" -s v -l verbose -d 'Print debug messages'
complete -c npins -n "__fish_npins_using_subcommand init" -s h -l help -d 'Print help'
complete -c npins -n "__fish_npins_using_subcommand add; and not __fish_seen_subcommand_from channel github forgejo gitlab git pypi container tarball help" -l name -d 'Add the pin with a custom name. If a pin with that name already exists, it will be overwritten' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and not __fish_seen_subcommand_from channel github forgejo gitlab git pypi container tarball help" -l frozen -d 'Add the pin as frozen, meaning that it will be ignored by `npins update` by default'
complete -c npins -n "__fish_npins_using_subcommand add; and not __fish_seen_subcommand_from channel github forgejo gitlab git pypi container tarball help" -s n -l dry-run -d 'Don\'t actually apply the changes'
complete -c npins -n "__fish_npins_using_subcommand add; and not __fish_seen_subcommand_from channel github forgejo gitlab git pypi container tarball help" -s v -l verbose -d 'Print debug messages'
complete -c npins -n "__fish_npins_using_subcommand add; and not __fish_seen_subcommand_from channel github forgejo gitlab git pypi container tarball help" -s h -l help -d 'Print help'
complete -c npins -n "__fish_npins_using_subcommand add; and not __fish_seen_subcommand_from channel github forgejo gitlab git pypi container tarball help" -f -a "channel" -d 'Track a Nix channel'
complete -c npins -n "__fish_npins_using_subcommand add; and not __fish_seen_subcommand_from channel github forgejo gitlab git pypi container tarball help" -f -a "github" -d 'Track a GitHub repository'
complete -c npins -n "__fish_npins_using_subcommand add; and not __fish_seen_subcommand_from channel github forgejo gitlab git pypi container tarball help" -f -a "forgejo" -d 'Track a Forgejo repository'
complete -c npins -n "__fish_npins_using_subcommand add; and not __fish_seen_subcommand_from channel github forgejo gitlab git pypi container tarball help" -f -a "gitlab" -d 'Track a GitLab repository'
complete -c npins -n "__fish_npins_using_subcommand add; and not __fish_seen_subcommand_from channel github forgejo gitlab git pypi container tarball help" -f -a "git" -d 'Track a git repository'
complete -c npins -n "__fish_npins_using_subcommand add; and not __fish_seen_subcommand_from channel github forgejo gitlab git pypi container tarball help" -f -a "pypi" -d 'Track a package on PyPi'
complete -c npins -n "__fish_npins_using_subcommand add; and not __fish_seen_subcommand_from channel github forgejo gitlab git pypi container tarball help" -f -a "container" -d 'Track an OCI container'
complete -c npins -n "__fish_npins_using_subcommand add; and not __fish_seen_subcommand_from channel github forgejo gitlab git pypi container tarball help" -f -a "tarball" -d 'Track a tarball'
complete -c npins -n "__fish_npins_using_subcommand add; and not __fish_seen_subcommand_from channel github forgejo gitlab git pypi container tarball help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from channel" -l name -d 'Add the pin with a custom name. If a pin with that name already exists, it will be overwritten' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from channel" -l frozen -d 'Add the pin as frozen, meaning that it will be ignored by `npins update` by default'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from channel" -s v -l verbose -d 'Print debug messages'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from channel" -s h -l help -d 'Print help'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from github" -s b -l branch -d 'Track a branch instead of a release' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from github" -l at -d 'Use a specific commit/release instead of the latest. This may be a tag name, or a git revision when --branch is set' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from github" -l upper-bound -d 'Bound the version resolution. For example, setting this to "2" will restrict updates to 1.X versions. Conflicts with the --branch option' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from github" -l release-prefix -d 'Optional prefix required for each release name / tag. For example, setting this to "release/" will only consider those that start with that string' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from github" -l name -d 'Add the pin with a custom name. If a pin with that name already exists, it will be overwritten' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from github" -l pre-releases -d 'Also track pre-releases. Conflicts with the --branch option'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from github" -l submodules -d 'Also fetch submodules'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from github" -l frozen -d 'Add the pin as frozen, meaning that it will be ignored by `npins update` by default'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from github" -s v -l verbose -d 'Print debug messages'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from github" -s h -l help -d 'Print help'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from forgejo" -s b -l branch -d 'Track a branch instead of a release' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from forgejo" -l at -d 'Use a specific commit/release instead of the latest. This may be a tag name, or a git revision when --branch is set' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from forgejo" -l upper-bound -d 'Bound the version resolution. For example, setting this to "2" will restrict updates to 1.X versions. Conflicts with the --branch option' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from forgejo" -l release-prefix -d 'Optional prefix required for each release name / tag. For example, setting this to "release/" will only consider those that start with that string' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from forgejo" -l name -d 'Add the pin with a custom name. If a pin with that name already exists, it will be overwritten' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from forgejo" -l pre-releases -d 'Also track pre-releases. Conflicts with the --branch option'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from forgejo" -l submodules -d 'Also fetch submodules'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from forgejo" -l frozen -d 'Add the pin as frozen, meaning that it will be ignored by `npins update` by default'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from forgejo" -s v -l verbose -d 'Print debug messages'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from forgejo" -s h -l help -d 'Print help'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from gitlab" -l server -d 'Use a self-hosted GitLab instance instead' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from gitlab" -l private-token -d 'Use a private token to access the repository.' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from gitlab" -s b -l branch -d 'Track a branch instead of a release' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from gitlab" -l at -d 'Use a specific commit/release instead of the latest. This may be a tag name, or a git revision when --branch is set' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from gitlab" -l upper-bound -d 'Bound the version resolution. For example, setting this to "2" will restrict updates to 1.X versions. Conflicts with the --branch option' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from gitlab" -l release-prefix -d 'Optional prefix required for each release name / tag. For example, setting this to "release/" will only consider those that start with that string' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from gitlab" -l name -d 'Add the pin with a custom name. If a pin with that name already exists, it will be overwritten' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from gitlab" -l pre-releases -d 'Also track pre-releases. Conflicts with the --branch option'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from gitlab" -l submodules -d 'Also fetch submodules'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from gitlab" -l frozen -d 'Add the pin as frozen, meaning that it will be ignored by `npins update` by default'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from gitlab" -s v -l verbose -d 'Print debug messages'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from gitlab" -s h -l help -d 'Print help'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from git" -l forge -r -f -a "none\t'A generic git pin, with no further information'
auto\t'Try to determine the Forge from the given url, potentially by probing the server'
gitlab\t'A Gitlab forge, e.g. gitlab.com'
github\t'A Github forge, i.e. github.com'
forgejo\t'A Forgejo forge, e.g. forgejo.org'"
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from git" -s b -l branch -d 'Track a branch instead of a release' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from git" -l at -d 'Use a specific commit/release instead of the latest. This may be a tag name, or a git revision when --branch is set' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from git" -l upper-bound -d 'Bound the version resolution. For example, setting this to "2" will restrict updates to 1.X versions. Conflicts with the --branch option' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from git" -l release-prefix -d 'Optional prefix required for each release name / tag. For example, setting this to "release/" will only consider those that start with that string' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from git" -l name -d 'Add the pin with a custom name. If a pin with that name already exists, it will be overwritten' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from git" -l pre-releases -d 'Also track pre-releases. Conflicts with the --branch option'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from git" -l submodules -d 'Also fetch submodules'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from git" -l frozen -d 'Add the pin as frozen, meaning that it will be ignored by `npins update` by default'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from git" -s v -l verbose -d 'Print debug messages'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from git" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from pypi" -l at -d 'Use a specific release instead of the latest' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from pypi" -l upper-bound -d 'Bound the version resolution. For example, setting this to "2" will restrict updates to 1.X versions. Conflicts with the --branch option' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from pypi" -l name -d 'Add the pin with a custom name. If a pin with that name already exists, it will be overwritten' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from pypi" -l frozen -d 'Add the pin as frozen, meaning that it will be ignored by `npins update` by default'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from pypi" -s v -l verbose -d 'Print debug messages'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from pypi" -s h -l help -d 'Print help'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from container" -l name -d 'Add the pin with a custom name. If a pin with that name already exists, it will be overwritten' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from container" -l frozen -d 'Add the pin as frozen, meaning that it will be ignored by `npins update` by default'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from container" -s v -l verbose -d 'Print debug messages'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from container" -s h -l help -d 'Print help'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from tarball" -l name -d 'Add the pin with a custom name. If a pin with that name already exists, it will be overwritten' -r -f
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from tarball" -l frozen -d 'Add the pin as frozen, meaning that it will be ignored by `npins update` by default'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from tarball" -s v -l verbose -d 'Print debug messages'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from tarball" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from help" -f -a "channel" -d 'Track a Nix channel'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from help" -f -a "github" -d 'Track a GitHub repository'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from help" -f -a "forgejo" -d 'Track a Forgejo repository'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from help" -f -a "gitlab" -d 'Track a GitLab repository'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from help" -f -a "git" -d 'Track a git repository'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from help" -f -a "pypi" -d 'Track a package on PyPi'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from help" -f -a "container" -d 'Track an OCI container'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from help" -f -a "tarball" -d 'Track a tarball'
complete -c npins -n "__fish_npins_using_subcommand add; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c npins -n "__fish_npins_using_subcommand show" -s p -l plain -d 'Prints only pin names'
complete -c npins -n "__fish_npins_using_subcommand show" -s e -l exclude -d 'Invert [NAMES] to exclude specified pins'
complete -c npins -n "__fish_npins_using_subcommand show" -s v -l verbose -d 'Print debug messages'
complete -c npins -n "__fish_npins_using_subcommand show" -s h -l help -d 'Print help'
complete -c npins -n "__fish_npins_using_subcommand update" -l max-concurrent-downloads -d 'Maximum number of simultaneous downloads' -r -f
complete -c npins -n "__fish_npins_using_subcommand update" -s p -l partial -d 'Don\'t update versions, only re-fetch hashes'
complete -c npins -n "__fish_npins_using_subcommand update" -s f -l full -d 'Re-fetch hashes even if the version hasn\'t changed. Useful to make sure the derivations are in the Nix store'
complete -c npins -n "__fish_npins_using_subcommand update" -s n -l dry-run -d 'Print the diff, but don\'t write back the changes'
complete -c npins -n "__fish_npins_using_subcommand update" -l frozen -d 'Allow updating frozen pins, which would otherwise be ignored'
complete -c npins -n "__fish_npins_using_subcommand update" -s v -l verbose -d 'Print debug messages'
complete -c npins -n "__fish_npins_using_subcommand update" -s h -l help -d 'Print help'
complete -c npins -n "__fish_npins_using_subcommand verify" -l max-concurrent-downloads -d 'Maximum number of simultaneous downloads' -r -f
complete -c npins -n "__fish_npins_using_subcommand verify" -s v -l verbose -d 'Print debug messages'
complete -c npins -n "__fish_npins_using_subcommand verify" -s h -l help -d 'Print help'
complete -c npins -n "__fish_npins_using_subcommand upgrade" -s v -l verbose -d 'Print debug messages'
complete -c npins -n "__fish_npins_using_subcommand upgrade" -s h -l help -d 'Print help'
complete -c npins -n "__fish_npins_using_subcommand remove" -s v -l verbose -d 'Print debug messages'
complete -c npins -n "__fish_npins_using_subcommand remove" -s h -l help -d 'Print help'
complete -c npins -n "__fish_npins_using_subcommand import-niv" -s n -l name -d 'Only import one entry from Niv' -r -f
complete -c npins -n "__fish_npins_using_subcommand import-niv" -s v -l verbose -d 'Print debug messages'
complete -c npins -n "__fish_npins_using_subcommand import-niv" -s h -l help -d 'Print help'
complete -c npins -n "__fish_npins_using_subcommand import-flake" -s n -l name -d 'Only import one entry from the flake' -r -f
complete -c npins -n "__fish_npins_using_subcommand import-flake" -s v -l verbose -d 'Print debug messages'
complete -c npins -n "__fish_npins_using_subcommand import-flake" -s h -l help -d 'Print help'
complete -c npins -n "__fish_npins_using_subcommand freeze" -s v -l verbose -d 'Print debug messages'
complete -c npins -n "__fish_npins_using_subcommand freeze" -s h -l help -d 'Print help'
complete -c npins -n "__fish_npins_using_subcommand unfreeze" -s v -l verbose -d 'Print debug messages'
complete -c npins -n "__fish_npins_using_subcommand unfreeze" -s h -l help -d 'Print help'
complete -c npins -n "__fish_npins_using_subcommand get-path" -s v -l verbose -d 'Print debug messages'
complete -c npins -n "__fish_npins_using_subcommand get-path" -s h -l help -d 'Print help'
complete -c npins -n "__fish_npins_using_subcommand help; and not __fish_seen_subcommand_from init add show update verify upgrade remove import-niv import-flake freeze unfreeze get-path help" -f -a "init" -d 'Intializes the npins directory. Running this multiple times will restore/upgrade the `default.nix` and never touch your sources.json'
complete -c npins -n "__fish_npins_using_subcommand help; and not __fish_seen_subcommand_from init add show update verify upgrade remove import-niv import-flake freeze unfreeze get-path help" -f -a "add" -d 'Adds a new pin entry'
complete -c npins -n "__fish_npins_using_subcommand help; and not __fish_seen_subcommand_from init add show update verify upgrade remove import-niv import-flake freeze unfreeze get-path help" -f -a "show" -d 'Lists the current pin entries'
complete -c npins -n "__fish_npins_using_subcommand help; and not __fish_seen_subcommand_from init add show update verify upgrade remove import-niv import-flake freeze unfreeze get-path help" -f -a "update" -d 'Updates all or the given pins to the latest version'
complete -c npins -n "__fish_npins_using_subcommand help; and not __fish_seen_subcommand_from init add show update verify upgrade remove import-niv import-flake freeze unfreeze get-path help" -f -a "verify" -d 'Verifies that all or the given pins still have correct hashes. This is like `update --partial --dry-run` and then checking that the diff is empty'
complete -c npins -n "__fish_npins_using_subcommand help; and not __fish_seen_subcommand_from init add show update verify upgrade remove import-niv import-flake freeze unfreeze get-path help" -f -a "upgrade" -d 'Upgrade the sources.json and default.nix to the latest format version. This may occasionally break Nix evaluation!'
complete -c npins -n "__fish_npins_using_subcommand help; and not __fish_seen_subcommand_from init add show update verify upgrade remove import-niv import-flake freeze unfreeze get-path help" -f -a "remove" -d 'Removes one pin entry'
complete -c npins -n "__fish_npins_using_subcommand help; and not __fish_seen_subcommand_from init add show update verify upgrade remove import-niv import-flake freeze unfreeze get-path help" -f -a "import-niv" -d 'Try to import entries from Niv'
complete -c npins -n "__fish_npins_using_subcommand help; and not __fish_seen_subcommand_from init add show update verify upgrade remove import-niv import-flake freeze unfreeze get-path help" -f -a "import-flake" -d 'Try to import entries from flake.lock'
complete -c npins -n "__fish_npins_using_subcommand help; and not __fish_seen_subcommand_from init add show update verify upgrade remove import-niv import-flake freeze unfreeze get-path help" -f -a "freeze" -d 'Freeze a pin entry'
complete -c npins -n "__fish_npins_using_subcommand help; and not __fish_seen_subcommand_from init add show update verify upgrade remove import-niv import-flake freeze unfreeze get-path help" -f -a "unfreeze" -d 'Thaw a pin entry'
complete -c npins -n "__fish_npins_using_subcommand help; and not __fish_seen_subcommand_from init add show update verify upgrade remove import-niv import-flake freeze unfreeze get-path help" -f -a "get-path" -d 'Evaluates the store path to a pin, fetching it if necessary. Don\'t forget to add a GC root'
complete -c npins -n "__fish_npins_using_subcommand help; and not __fish_seen_subcommand_from init add show update verify upgrade remove import-niv import-flake freeze unfreeze get-path help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c npins -n "__fish_npins_using_subcommand help; and __fish_seen_subcommand_from add" -f -a "channel" -d 'Track a Nix channel'
complete -c npins -n "__fish_npins_using_subcommand help; and __fish_seen_subcommand_from add" -f -a "github" -d 'Track a GitHub repository'
complete -c npins -n "__fish_npins_using_subcommand help; and __fish_seen_subcommand_from add" -f -a "forgejo" -d 'Track a Forgejo repository'
complete -c npins -n "__fish_npins_using_subcommand help; and __fish_seen_subcommand_from add" -f -a "gitlab" -d 'Track a GitLab repository'
complete -c npins -n "__fish_npins_using_subcommand help; and __fish_seen_subcommand_from add" -f -a "git" -d 'Track a git repository'
complete -c npins -n "__fish_npins_using_subcommand help; and __fish_seen_subcommand_from add" -f -a "pypi" -d 'Track a package on PyPi'
complete -c npins -n "__fish_npins_using_subcommand help; and __fish_seen_subcommand_from add" -f -a "container" -d 'Track an OCI container'
complete -c npins -n "__fish_npins_using_subcommand help; and __fish_seen_subcommand_from add" -f -a "tarball" -d 'Track a tarball'
