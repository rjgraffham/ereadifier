# ereadifier

Single-purpose rust server that postprocesses images downloaded
by suwayomi, to make them more suitable for reading on ereaders.

It can work with any reader which supports WebP decoding, though
it is developed primarily for Koreader.

It listens for any POST requests, receives an input image via
in the `image` form field, optionally resizes it to fit within
the configured dimensions, and then responds with the final
image encoded as a WebP.

## Configuration

ereadifier is configured strictly through environment variables,
as it is intended to be deployed as a container. The following
configuration options are available:

* `EREADIFIER_DIMENSIONS`
    * `""` (empty string) - Do not resize. This is the
      **default** behaviour.
    * `<width>x<height>` - Resize to fit within a `<width>` by
      `<height>` area.
    * `<preset>` - Resize to a named preset size. A list of
      device presets is available [here](#device-presets)
* `EREADIFIER_ENCODE`
    * `lossless` - Always encode as a lossless WebP. This is
      the **default** behaviour.
    * `lossy` - Always encode as a lossy WebP. See below to
      configure the target quality.
    * `smallest` - Encode as both lossless and lossy WebP, and
      select whichever has a smaller file size. This is slower
      due to encoding twice, but can be useful with sources
      which have blank pages (typically inserted to ensure an
      even page count for double page reading), as a fully
      blank page compresses significantly better as a
      lossless WebP.
* `EREADIFIER_LOSSY_QUALITY`
    * `<quality>` - Set the target quality for lossy encodes
      to `<quality>`. Valid range is 0 through 100, with a default
      of 85.
* `EREADIFIER_DOUBLE_PAGE`
    * `ignore` - Do not attempt to detect double pages. All
      images will target the same dimensions.
    * `wider` - If the provided image has a width greater than
      its height, assume it's a double page, and use twice the
      target width. This does not split the page, as suwayomi
      only expects one image out, but it should produce images
      similar in scale to if the pages had been split.
    * `much_wider` - As with `wider`, except that the threshold
      for being detected as a double page is a width at least 50%
      larger than height (i.e., 3:2 aspect or wider). This is
      the **default** behaviour.
* `EREADIFIER_LISTEN`
    * `<address>:<port>` - Address and port to listen on.
      Address *must* be a valid IPv4 or IPv6 address (not a
      hostname), and IPv6 addresses *must* be wrapped in `[`
      and `]`. Default is `0.0.0.0:80`.

## Device Presets

The following device presets and dimensions are currently
available (largest to smallest):

* `scribe2025` - 1980 x 2640
    * Kindle Scribe Colorsoft (1st Generation)
    * Kindle Scribe (3rd Generation)
* `scribe` - 1860 x 2480
    * Kindle Scribe (and 2024 release)
* `forma`, `sage` - 1440 x 1920
    * Kobo Forma
    * Kobo Sage
* `auraone`, `ellipsa` - 1404 x 1872
    * Kobo Aura One
    * Kobo Ellipsa (all variants)
* `libra`, `oasis2018`, `paperwhite2024`, `colorsoft` - 1264 x 1680
    * Kobo Libra (all variants)
    * Kindle Oasis (9th and 10th Generation)
    * Kindle Paperwhite (12th Generation, including Signature Edition)
    * Kindle Colorsoft (1st Generation, including Signature Edition)
* `paperwhite2021` - 1236 x 1648
    * Kindle Paperwhite (11th Generation, including Signature Edition)
* `aurah2o`, `aurahd` - 1080 x 1440
    * Kobo Aura H2O (and 2nd Edition)
    * Kobo Aura HD
* `clara`, `glohd`, `voyage`, `paperwhite2015`, `oasis`, `kindle2022` - 1072 x 1448
    * Kobo Clara (all variants)
    * Kobo Glo HD
    * Kindle Voyage (7th Generation)
    * Kindle Paperwhite (7th and 10th Generation)
    * Kindle Oasis (8th Generation)
    * Kindle (11th Generation)
* `kindledx` - 824 x 1200
    * Kindle DX (2nd Generation)
* `aura`, `glo` - 768 x 1024
    * Kobo Aura (and 2nd Edition)
    * Kobo Glo
* `nia`, `paperwhite` - 758 x 1024
    * Kobo Nia
    * Kindle Paperwhite (5th and 6th Generation)
* `kobo`, `kindle` - 600 x 800
    * Kobo Original
    * Kobo Mini
    * Kobo Touch (and Touch 2.0)
    * Kobo WiFi
    * Kindle (1st Generation through 10th Generation)
    * Kindle Keyboard (3rd Generation)
    * Kindle Touch (4th Generation)
