{ pkgs, lib, config, inputs, ... }:

{
  packages = with pkgs; [ git alsa-lib udev wayland pkg-config libGL ];
  languages.rust = { enable = true; channel = "nightly"; };
  env.LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [ pkgs.libxkbcommon pkgs.vulkan-loader ];
}
