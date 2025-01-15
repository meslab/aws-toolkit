# AWS toolkit

This toolset has some command line utilities which help for day-to-day operations on AWS.

```
ssm-session
scale-in-ecs
ses-suppression-list
ecr-gitconfig
release-codepipelines
```

## How to use

Each command has `--help` section
```bash
ssm-session --help
scale-in-ecs --help
ses-suppression-list --help
ecr-gitconfig --help
release-codepipelines --help
```

## Installation 

Linux:
```bash
git clone https://github.com/meslab/aws-toolkit.git
cd aws-toolkit/
make install
```