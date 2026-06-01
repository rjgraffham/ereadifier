# Roadmap

Improvements I want to make, in no particular order:

* Support processing images for landscape orientation reading - this will take some thinking about how double page behaviour should change in this situation
* Untangle various remaining eprintln! uses in order to more consistently apply the debug mode
    * Additionally, move all "config in use" (including the preset used) prints into the print_config function, as currently if a preset is active, it prints in the healthcheck despite the listen address being the only configuration used there.