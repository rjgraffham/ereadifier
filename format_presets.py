#!/usr/bin/env python

def flatten(xss):
    return [x for xs in xss for x in xs]

def main():
    import os.path
    import tomllib
    from collections import defaultdict

    script_dir = os.path.dirname(os.path.realpath(__file__))
    presets_path = os.path.join(script_dir, "presets.toml")

    presets_by_size = defaultdict(lambda: [])

    with open(presets_path, "rb") as f:
        presets = tomllib.load(f)
        for preset_name, preset in presets.items():
            presets_by_size[(preset["width"], preset["height"])].append((preset_name, preset["devices"]))
    
        # PRESETS.md header
        print("# Device Presets")
        print()
        print("The following device presets and dimensions are currently")
        print("available (largest to smallest):")
        print()

        # PRESETS.md list
        for dims in reversed(sorted(presets_by_size.keys())):  # largest WxH first
            (width, height) = dims
            presets_at_size = sorted(presets_by_size[dims])    # lexically sorted preset names
            preset_names = [p[0] for p in presets_at_size]
            preset_devices = flatten([p[1] for p in presets_at_size])
            presets_str = ", ".join(map(lambda p: f"`{p}`", preset_names))
            print(f"* {presets_str} - {width} x {height}")
            for device in preset_devices:
                print(f"    * {device}")
            
        
        # PRESETS.md footer (currently blank line)
        print()

if __name__ == '__main__':
    main()