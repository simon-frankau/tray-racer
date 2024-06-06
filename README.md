# Tray Racer: A simple curved-space ray tracer

The aim of this project is to render a simple wormhole by ray-tracing
the curved rays through it. No fancy physics, no geometry, just a
couple of environment maps on each side, but hopefully enough to give
you the idea.

Based on the maths discussed in
https://github.com/simon-frankau/curved-spaces/ .

## TODO

At this stage, everything! Simple initial steps are:

 * Complete software environment mapping.
 * Start tracing rays into the environment map...

## Design choices

For interactive use, I'm reusing the egui/glow/winit/glut code that I
used before for
[curved-spaces](https://github.com/simon-frankau/curved-spaces/),
since it gives me a simple starting point. I've ripped out the support
for wasm because, even though the maintenance cost was pretty minimal,
I don't really care about it right now. I've removed it in a way that
shouldn't make it too hard to add back in later.

## Environment maps

The environment maps were sourced from
https://opengameart.org/content/skybox .
