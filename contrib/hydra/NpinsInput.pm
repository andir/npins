package Hydra::Plugin::NpinsInput;

use strict;
use warnings;
use parent 'Hydra::Plugin';
use Carp;
use File::Path;
use Hydra::Helper::Nix;
use Nix::Store;
use Fcntl qw(:flock);
use Digest::SHA qw(sha256_hex);
use File::Slurper qw(read_text);
use JSON::MaybeXS qw(decode_json);
use Data::Dumper;

## no critic (InputOutput::RequireBriefOpen)
## no critic (CodeLayout::ProhibitParensWithBuiltins)
## no critic (ControlStructures::ProhibitCascadingIfElse)
## no critic (NamingConventions::Capitalization)
## no critic (Subroutines::ProhibitExcessComplexity, Subroutines::ProhibitManyArgs)
## no critic (ValuesAndExpressions::ProhibitConstantPragma)

our $VERSION = 1;
use constant DEFAULT_GIT_TIMEOUT => 600;

sub _print_if_debug {
    my ($evaluation, $msg) = @_;

    if (not $ENV{'HYDRA_DEBUG'}) {
        return;
    }

    my $evalid = $evaluation->id;
    my $project = $evaluation->jobset->project->name;
    my $jobset = $evaluation->jobset->name;

    print {*STDERR} "NpinsInput: [Eval $evalid of $project:$jobset] $msg\n" || croak;
    return;
}

sub _print {
    my ($evaluation, $msg) = @_;
    my $evalid = $evaluation->id;
    my $project = $evaluation->jobset->project->name;
    my $jobset = $evaluation->jobset->name;

    print {*STDERR} "NpinsInput: [Eval $evalid of $project:$jobset] $msg\n" || croak;
    return;
}

sub supportedInputTypes {
    my ($self, $input_types) = @_;
    $input_types->{'npins-channel'} = 'Npins channel';
    $input_types->{'npins-pypi'} = 'Npins PyPi';
    return;
}

sub evalAdded {
    my ($self, $trace_id, $jobset, $evaluation) = @_;

    # Find pins directory we need to act on
    my $npins_dir_input = $evaluation->jobsetevalinputs->find({ name => 'npins-directory', type => 'string' });
    my $npins_dir_name = 'npins';
    if (defined($npins_dir_input)) {
        $npins_dir_name = $npins_dir_input->get_column('value');
    }

    # Find GitHub host
    my $github_host_input = $evaluation->jobsetevalinputs->find({ name => 'npins-github-host', type => 'string' });
    my $github_host = 'https://github.com';
    if (defined($github_host_input)) {
        $github_host = $github_host_input->get_column('value');
    }

    # Find the primary input and its sources.json
    my $repo_root = $evaluation->jobsetevalinputs->find({ name => $evaluation->nixexprinput, altnr => 0 });
    if (not defined($repo_root)) {
        _print_if_debug($evaluation, 'Skipping as there does not seem to be a main input');
        return;
    }
    $repo_root = $repo_root->path;
    my $npins_dir = "$repo_root/$npins_dir_name";
    my $sources_json = "$npins_dir/sources.json";
    _print_if_debug($evaluation, "Will look for sources in $sources_json");
    if (not -f $sources_json) {
        _print_if_debug($evaluation, "No sources.json in $npins_dir_name/");
        return;
    }

    # Parse JSON
    my $json_contents;
    if (not eval { $json_contents = decode_json(read_text($sources_json)); 1 }) {
        _print($evaluation, 'Invalid JSON');
        return;
    }

    if ($json_contents->{version} != 2) {
        _print($evaluation, 'Unexpected sources.json version ' . $json_contents->{version});
        return;
    }

    # List inputs and insert them
    keys %{$json_contents};
    while (my ($pin_name, $pin_config) = each %{$json_contents->{pins}}) {
        # Do not do anything if an input with that name already exists
        if (defined($evaluation->jobsetevalinputs->find({ name => $pin_name }))) {
            _print_if_debug($evaluation, "Skipping $pin_name because an input with that name already exists");
            next;
        }

        my %jobsetevalinput = (
            name => $pin_name,
            altnr => 0,
            sha256hash => $pin_config->{hash},
        );
        if ($pin_config->{type} eq 'Channel') {
            $jobsetevalinput{uri} = $pin_config->{url};
            $jobsetevalinput{type} = 'npins-channel';
        } elsif ($pin_config->{type} eq 'Git' or $pin_config->{type} eq 'GitRelease') {
            my $revision = $pin_config->{revision};
            if ($pin_config->{repository}->{type} eq 'Git') {
                $jobsetevalinput{uri} = $pin_config->{repository}->{url};
            } elsif ($pin_config->{repository}->{type} eq 'GitHub') {
                $jobsetevalinput{uri} = "$github_host/$pin_config->{repository}->{owner}/$pin_config->{repository}->{repo}.git";
            } elsif ($pin_config->{repository}->{type} eq 'GitLab') {
                $jobsetevalinput{uri} = "$pin_config->{repository}->{server}/$pin_config->{repository}->{repo_path}";
            } else {
                _print($evaluation, "Unknown git repo type $pin_config->{repository}->{type}");
                next;
            }

            my $cfg = $self->{config}->{'git-input'};
            my $timeout = DEFAULT_GIT_TIMEOUT;
            my $umask;
            if (defined($cfg)) {
                $timeout = $cfg->{timeout} // DEFAULT_GIT_TIMEOUT;
                $umask = $cfg->{umask} // undef;
            }

            # Set desired umask
            my $old_umask;
            if (defined($umask)) {
                $old_umask = umask();
                umask($umask);
            }

            # Clone or update a branch of the repository into our SCM cache.
            my $cache_dir = getSCMCacheDir() . '/git';
            mkpath($cache_dir);
            my $clone_path = $cache_dir . q{/} . sha256_hex($jobsetevalinput{uri});
            _print_if_debug($evaluation, "Using $clone_path as git repository");

            open(my $lock, '>', "$clone_path.lock") or croak('Could not open git lock file');
            flock($lock, LOCK_EX) or croak('Could not lock git lock');

            my $res;
            if (! -d $clone_path) {
                # Clone everything and fetch the branch.
                $res = run(cmd => ['git', 'init', $clone_path]);
                if ($res->{status}) {
                    _print($evaluation, "Error creating git repo (rc $res->{status}): $res->{stderr}");
                    next;
                }
                $res = run(cmd => ['git', 'remote', 'add', 'origin', q{--}, $jobsetevalinput{uri}], dir => $clone_path);
                if ($res->{status}) {
                    _print($evaluation, "Error adding remote to git repository (rc $res->{status}): $res->{stderr}");
                    next;
                }
            }

            # This command forces the update of the local branch to be in the same as
            # the remote branch for whatever the repository state is.  This command mirrors
            # only one branch of the remote repository.
            $res = run(cmd => ['git', 'fetch', '-fu', 'origin', "+$pin_config->{revision}:_hydra_tmp"], dir => $clone_path, timeout => $timeout);
            if ($res->{status}) {
                $res = run(cmd => ['git', 'fetch', '-fu', 'origin'], dir => $clone_path, timeout => $timeout);
                if ($res->{status}) {
                    _print($evaluation, "Error fetching git repository (rc $res->{status}): $res->{stderr}");
                    next;
                }
            }

            # Get store path
            _print_if_debug($evaluation, "Will try to instantiate $npins_dir");
            $res = run(cmd => ['nix', 'eval', '--impure', '--raw', '--expr', "(import $npins_dir).\"$pin_name\""]);
            if ($res->{status}) {
                _print($evaluation, "Unable to nix eval npins (rc $res->{status}): $res->{stderr}");
            } else {
                $jobsetevalinput{path} = $res->{stdout};
                addTempRoot($res->{stdout});
            }

            $jobsetevalinput{revision} = $revision;
            $jobsetevalinput{type} = 'git';

            if (defined($old_umask)) {
                umask($old_umask);
            }

        } elsif ($pin_config->{type} eq 'PyPi') {
            $jobsetevalinput{type} = 'npins-pypi';
            $jobsetevalinput{uri} = $pin_config->{url};
        } else {
            _print($evaluation, 'Unknown pin type ' . $pin_config->{type});
            next;
        }

        _print_if_debug($evaluation, 'Adding input: ' . Dumper(\%jobsetevalinput));
        $evaluation->jobsetevalinputs->create(\%jobsetevalinput);
    }

    return;
}

1;
