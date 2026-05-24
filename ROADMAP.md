# Roadmap

Improvements I want to make, in no particular order:

* Add a health check endpoint (and move conversion to an explicit /convert endpoint)
* Return structured JSON errors
* Make presets data-driven to ease expanding the list later:
    - TOML file which defines the dimensions and a list of devices for each named preset
    - Server loads TOML file when parsing dimensions
    - PRESETS.md generated from TOML file on pre-commit, instead of the current manually written version
* Support processing images for landscape orientation reading - this will take some thinking about how double page behaviour should change in this situation