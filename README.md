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
      device presets is available [here](PRESETS.md)
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

## Why WebP?

As this is built primarily for my own personal use, I can rely
on the assumption that all readers will support WebP, and
generally WebP at a similar visual quality will be smaller
than JPEG or PNG, and those space savings add up with a device
like an eReader which only has a small amount of onboard storage.