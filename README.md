## 🌟 Overview

My opinionated base configuration for Nix flake setups, providing a foundational layer of development tools and settings for macOS. This repository focuses on nix-darwin and home-manager configurations that universally apply across different contexts - personal projects, work environments, or anywhere you need a consistent development setup.

Think of it as your "baseline development environment" - the tools and settings you always want available, configured exactly how you like them, without any context-specific data like email addresses or API keys.

## 📁 Project Structure

```
core.nix/
├── darwin/
│   └── default.nix      # macOS system preferences
├── home-manager/
│   ├── default.nix      # Imports all modules
│   ├── neovim/          # Editor with plugins
│   │   ├── default.nix
│   │   └── plugins/
│   ├── zsh/             # Shell with completions
│   │   ├── default.nix
│   │   └── init.zsh
│   └── ...              # Each tool in its own module
├── flake.nix            # Module exports
└── LICENSE              # MIT license
```

# 🚀 How To Use

Add this as a flake input to your Nix configuration:

```nix
{
  inputs = {
    core.url = "github:Jitsusama/core.nix";
    # ... other inputs
  };

  outputs = { self, core, ... }@inputs: {
    # Option 1: Import everything (recommended to start)
    darwinConfigurations.hostname = {
      imports = [
        core.darwinModule
      ];
    };
    homeConfigurations.username = {
      imports = [
        core.homeManagerModule
      ];
    };

    # Option 2: Import only what you need
    homeConfigurations.username = {
      imports = [
        # Pick specific modules from core
        core.homeManagerModule.neovim
        core.homeManagerModule.git
        core.homeManagerModule.zsh
      ];
    };
  };
}
```

Then add your context-specific configuration on top:

```nix
homeConfigurations.username = {
  imports = [
    core.homeManagerModule
    # Your additional modules
  ];
  
  # Add your git identity
  programs.git = {
    userEmail = "your.email@example.com";
    userName = "Your Name";
  };
  
  # Add work-specific or personal tools
  home.packages = with pkgs; [
    internal-tool
    company-cli
  ];
}
```

