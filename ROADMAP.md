# Roadmap

Improvements I want to make, in no particular order:

* Make presets data-driven to ease expanding the list later:
    - TOML file which defines the dimensions and a list of devices for each named preset
    - Server loads TOML file when parsing dimensions
    - PRESETS.md generated from TOML file on pre-commit ([synced hooks][hooks]), instead of the current manually written version
        * Another option would be to have a workflow that rebuilds PRESETS.md on changes to the TOML - this would mean
          a revision exists where they're desynced, but the gap would be very small, and using a workflow would remove
          the need to sync hooks between instances of the repo, the need to ensure any tooling used in that build is
          installed on all hosts, etc.
* Support processing images for landscape orientation reading - this will take some thinking about how double page behaviour should change in this situation
* Improve error handling - currently it seems that some errors I didn't expect are showing up as 500 Unknown
    - Further to this, implement a proper debug toggle:
        * Make current logging (except the config log) contingent on debug mode instead of printing all the time
        * Save out the input image to a temporary location on error if debug mode enabled, for examination if needed
    - Revisit areas where errors are currently either printed or suppressed (e.g., into Option), and bubble them up

[hooks]: https://www.viget.com/articles/two-ways-to-share-git-hooks-with-your-team