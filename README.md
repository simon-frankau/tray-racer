# Tray Racer: A simple curved-space ray tracer

The aim of this project is to render a simple wormhole by ray-tracing
the curved rays through it. No fancy physics, no geometry, just a
couple of environment maps on each side, but hopefully enough to give
you the idea.

Based on the maths discussed in
https://github.com/simon-frankau/curved-spaces/ .

The result is something that looks like this:

![Image of a beach with something that looks like a spherical lens in
the middle, giving a view through to a path at night. There is some
distortion around the sphere.](./tray-racer.png)

## Usage

Fetch some environment maps as described below. For interactive
exploring you can run

```
cargo run --release --bin tray-racer-app
```

If you want to generate images, you can use `tray-racer-cli`. An
example of using this to generate an animation can be found in
[pan.sh](./pan.sh). A pre-generated version is [here](./pan.mp4).

If you want to read up in tedious detail how I got adaptive
step-sizing working, you can read
[convergence-test/README.md](convergence-test/README.md).

## Next steps (aka TODO)

The basic stuff works pretty well now, and I even have adaptive
step-sizing. What I would like to do is be able to create an animation
of passing through the wormhole, though.

Once it works nicely, I should probably document how to run this
thing, too!

## Design choices

For interactive use, I'm reusing the egui/glow/winit/glut code that I
used before for
[curved-spaces](https://github.com/simon-frankau/curved-spaces/),
since it gives me a simple starting point. I've ripped out the support
for wasm because, even though the maintenance cost was pretty minimal,
I don't really care about it right now. I've removed it in a way that
shouldn't make it too hard to add back in later.

### Goals

 * **Get something working** I want pretty output. :)
 * **Learn through discovery** Actually a higher priority that "making
   it work". Along with the "curved-spaced" project, my goal is to
   derive the maths I need to make all this work from scratch, and
   explore anything else that crops up that interests me. While I
   could take formulae from a book without understanding them, the
   overall goal is to gain a stronger understanding by working it out
   for myself, and then checking it against the published maths
   afterwards.

I understand these are relatively personal goals: If you want pretty
output, my learning is unimportant. If you want to learn the maths,
you might be better off with a maths text. On the other hand, I hope
that by documenting what I've learnt, it might provide an insight to
people who think about this as I do. Good luck!

### Non-goals

 * **High performance** I'm not interested in O(1) performance
   improvements, even if that's "run insanely fast on a GPU". For this
   project I want to keep my code simple, and not spend time looking
   at detailed optimisations.
 * **Space-specific optimisation** The implicit surface is a simple
   polynomial, so its derivative can be calculated symbolically, but
   I'm still using finite difference to retain flexibility and knock
   out one source of complexity (albeit to replace it with other
   numerical stability issues). There could even be a closed form
   solution for the paths involved, but I'm not really interested in
   that.
 * **Anti-aliasing** You can get cheap (but poor) anti-aliasing by
   rendering at a higher resolution and downscaling.
 * **Automated tests** Yeah, I don't have a very good excuse on this
   one. I've been doing ad hoc testing by looking at the output along
   the way, and am not really expecting major changes that need tests
   to ensure nothing has fallen apart (beyond further ad hoc looking
   at the output!).

None of these goals are bad goals, but I have very little energy right
now, and want to keep it focused on simply making it do the basics.

**Note:** Yes, I have improved performance by using Rayon to
parallelise the main rendering, going against my non-goals. It was,
however, cheap and easy to do!

## Environment maps

The environment maps were sourced from
https://opengameart.org/content/skybox .
