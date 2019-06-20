The software is bundled with 2 different command line software:

1. **jormungandr**: the node;
2. **jcli**: Jormungandr Command Line Interface, the helpers and primitives to run and interact with the node.

# Help and auto completion

All commands come with usage help with the option `--help` or `-h`.

For `jcli`, it is possible to generate the auto completion with:

```
jcli auto-completion bash ${HOME}/.bash_completion.d
```

Supported shells are: bash, fish, zsh, powershell and elvish.

**Note:**
Make sure `${HOME}/.bash_completion.d` directory previously exists on your HD.
In order to use auto completion you still need to:
```
source ${HOME}/.bash_completion.d/jcli.bash
```
You can also put it in your `${HOME}/.bashrc`.
