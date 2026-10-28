# AWS toolkit

This toolset has some command line utilities which help for day-to-day operations on AWS.

```
ssm-session
scale-in-ecs
ses-suppression-list
ecr-gitconfig
release-codepipelines
s3-guardduty-copy
```

## How to use

Each command has a `--help` section:

```bash
ssm-session --help
scale-in-ecs --help
ses-suppression-list --help
ecr-gitconfig --help
release-codepipelines --help
s3-guardduty-copy --help
```

All commands use the standard AWS credential chain. Select a profile with
`--profile` (default: `default`) and a region with `--region` (default:
`eu-central-1`). Most listing commands accept `--prefix-match` / `-m` and
`--prefix-exclude` / `-x` filters; an empty include list matches everything.

### Examples

```bash
# Open a shell inside the container of an ECS service
ssm-session --cluster app --service auth

# Stop RDS instances matching "<cluster>-postgres"
scale-in-ecs --cluster app --scaledown

# Delete all cluster resources (requires confirmation when dropping databases
# without a final snapshot)
scale-in-ecs --cluster app --delete

# Export suppressed SES destinations
ses-suppression-list --region eu-central-1 --output suppressed_emails.csv

# Release all failed CodePipelines matching a prefix
release-codepipelines --failed-only --prefix-match my-app --dry-run

# Re-scan S3 objects missing the GuardDuty malware scan tag
s3-guardduty-copy --all --prefix-match my-bucket --dry-run
```

## Requirements

- Rust 1.88 or newer (the crate uses edition 2024 and let chains).
- AWS credentials available through the default credential chain (environment
  variables, shared config, SSO, or instance role).
- The `aws` CLI must be installed for `ssm-session`.
- For `ssm-session`, the target EC2 instance must be registered with SSM and the
  remote host must have Docker and sudo available.

## Development

```bash
make format   # cargo fmt
make lint     # cargo clippy
make test     # cargo test
make build    # cargo build
make release  # release builds of all binaries
```

## Installation

Linux:

```bash
git clone https://github.com/meslab/aws-toolkit.git
cd aws-toolkit/
make install
```

`make install` copies the release binaries into `~/.local/bin`.
