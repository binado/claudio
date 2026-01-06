I built this cli tool claudio. You can read what it does in @README.md .
I want to redesign the tool from using "providers" to using a more general "config" approach. The idea is that the user can have different configurations for different use cases, not necessarily tied to specific providers. The user may run the tool with different config files or settings depending on their needs.

Normally, this is done in claude code by passing the `--config-file` flag to specify a config file. My idea is more general: have a tool that runs claude with an arbitrary set of configuration files, cli flags, and environment variables.

My idea comes from the design of tools like television (the cli fuzzy-finder like fzf). Television allows users to specify different *channels*, which are recipes that are completely specified in a configuration file. Each channel can have its own settings, filters, and behaviors.

For instance, a user can have a "minimax" config that uses minimax as a provider (hence overwrites some environment variables), and may also want to disable mcps since the minimax model may have a smaller context window. Thinking of the UX, the user may run:

```bash
# claudio retrives the 'minimax' config and applies its settings
claudio run minimax
```

```bash
claudio run non-existent-config
# Error: Config 'non-existent-config' not found.
```

I want you to analyze this idea and provide feedback on its feasibility, potential challenges, and benefits. Additionally, suggest how the configuration files could be structured to allow for maximum flexibility and ease of use. Consider how this approach compares to the existing provider-based system in terms of user experience and maintainability.
