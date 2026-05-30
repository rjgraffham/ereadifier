# Roadmap

Improvements I want to make, in no particular order:

* Support processing images for landscape orientation reading - this will take some thinking about how double page behaviour should change in this situation
* Improve error handling - currently it seems that some errors I didn't expect are showing up as 500 Unknown
    - Further to this, implement a proper debug toggle:
        * Make current logging (except the config log) contingent on debug mode instead of printing all the time
        * Save out the input image to a temporary location on error if debug mode enabled, for examination if needed
    - Consider returning the input image on conversion failure, as currently the error outputs are being saved as files despite error codes
