
# Custom pin completions appended from here

# Our way of calling npins in fish while accounting for the lockfile/directory this completion is on
function __fish_npins
    set -l saved_args $argv
    set -l cmd (commandline -xpc)
    set -e cmd[1]
	argparse -s (__fish_npins_global_optspecs) -- $cmd 2>/dev/null; or return
	set -q _flag_lock_file; and set -l lockfile "--lock-file" $_flag_lock_file
	set -q _flag_directory; and set -l directory "--directory" $_flag_directory
	command npins $lockfile $directory $saved_args 2>/dev/null
end

# Provide completions for a single pin
function __fish_npins_pin_single
	# In options we need to have a empty description to override
	# the option's description which we inherit for some reason
	set -l joined "$(__fish_npins show -p | string join \t\n)"
	if test -n $joined 
		echo "$joined"\t
	end
end

# Provide completions for multiple pins, excluding already provided pins
function __fish_npins_pin_list
	set -l cmd (commandline -xpc)
	set -e cmd[1]
	# Remove out options and only keep the pin arguments
	# for example
	# update --dry-run nixpkgs --full home-manager
	# turns into
	# update nixpkgs home-manager
	argparse --unknown-arguments=none (__fish_npins_global_optspecs) -- $cmd 2>/dev/null; or return
	__fish_npins show -p -e $argv[2..]
end

# --name for all npins add subcommands and for npins add itself
complete -c npins -n "__fish_npins_using_subcommand add; and not __fish_seen_subcommand_from help" -l name -x -a '(__fish_npins_pin_single)'

# Commands which can be provided a pin list
complete -c npins -n "__fish_npins_using_subcommand show" -f -a '(__fish_npins_pin_list)'
complete -c npins -n "__fish_npins_using_subcommand update" -f -a '(__fish_npins_pin_list)'
complete -c npins -n "__fish_npins_using_subcommand verify" -f -a '(__fish_npins_pin_list)'

# Commands which require a pin list
complete -c npins -n "__fish_npins_using_subcommand remove" -x -a '(__fish_npins_pin_list)'
complete -c npins -n "__fish_npins_using_subcommand freeze" -x -a '(__fish_npins_pin_list)'
complete -c npins -n "__fish_npins_using_subcommand unfreeze" -x -a '(__fish_npins_pin_list)'
