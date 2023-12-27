# A LEDstrip flower for the RP-Pico 2040

This is a bad weather afternoon hack during last November that I did to
decorate [our Esperanto assembly](https://events.ccc.de/congress/2023/hub/en/assembly/esperanto/)
at the [37c3](https://events.ccc.de/congress/2023/infos/startpage.html).  Since
many that came across asked for it I publish the source code. It is now by no
means documented.  I am not sure if I am going to change that.  :)

So just some basic stuff …


## Idea

The idea was actually to make a 24 strip circle, not a half circle of 12
strips.  Unfortunately the second SPI of the RP-2040 chip ceased working. I
have to check it when I get home with my osci.


## Making of

I've chosen the RP-2040 for the project for two reasons.

* It has two SPIs (one of which is not working ☹️), as I was not able to run all
  the 24 strips on one SPI.

* The SPIs of the RP-2040 are said to deal with 5V quite well and as it turns
  out I have not experienced logic voltage issues – well maybe some small ones.

The LED strips are made of APA102 LEDs.  They work quite well with the SPI
busses of the RP-2040.  The APA102s are so-called 4-wire addressable LEDs.  Two
wires are for power supply and ground. The third for the signal clock and the
fourth for the signal.  One crucial point that I learned is that the signal
clock is not global but each LED regenerates the clock signal so that it does
not degenerate down the strip.  That is why there are two wires from the top of
each strip back to the bottom.


## Programming

The whole thing is written in Rust. Basically because I wanted to do something
with Embedded Rust.


Feel free to ask questions in the issue tracker.
