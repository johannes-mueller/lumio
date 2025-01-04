# A LED strip flower for the RP-Pico 2040

This is a bad weather afternoon hack during that I did in November 2023 to
decorate [our Esperanto assembly](https://events.ccc.de/congress/2023/hub/en/assembly/esperanto/)
at the [37c3](https://events.ccc.de/congress/2023/infos/startpage.html).  Since
many that came across asked for it, I've published the source code. It's now by no
means documented.  I'm not sure if I am going to change that.  :)

So just some basic stuff …


## Idea

The idea was actually to make a 24 strip circle, not a half circle of 12
strips.  The LED-strip are glued to a L-profile stick each.  These sticks are
then mounted with one end to a wooden disk and adjusted in a way that they
point upwards to form a funnel.  In this form the installation can show
interesting light effects.


## Making of

I've chosen the RP-2040 for the project for two reasons.

* It has two SPIs. The plan was to have 24 LED strips with 60 LEDs each, so
  1440 LEDs in total. So each SPI handles 720 LEDs.

* The SPIs of the RP-2040 are said to deal with 5V quite well and as it turns
  out, I haven't experienced logic voltage issues – well maybe some small ones.

The LED strips are made of APA102 LEDs.  They work quite well with the SPI
busses of the RP-2040.  The APA102s are so-called 4-wire addressable LEDs.  Two
wires are for power supply and ground.  The third for the signal clock and the
fourth for the signal.  One crucial point that I've learned is that the signal
clock is not global, but each LED regenerates the clock signal, so that it does
not degenerate down the strip.  That's why there are two wires from the top of
each strip back to the bottom.

As it turns out the whole setup can take a quite some energy.  Letting all the
LEDs shine completly white would pull way more than 10 A. That's why quite some
thick cables (2.5 mm²) are used to feed the LED strips.  This is not sufficient
if all the LEDS are full white.  In practice we don't have those bright
programs.  The usual ones just take 3 A to 4 A.


## Programming

The whole thing is written in Rust. Basically, because I wanted to do something
with Embedded Rust.


Feel free to ask questions in the issue tracker.
