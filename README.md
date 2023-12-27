# A ledstrip flower for the RP-Pico 2040

This is a bad whether afternoon hack that I did to decorate our assembly at the
37c3.  Since many that came across asked for it I publish the source code. It
is now by no means documented.  I am not sure if I am going to change that :)

Just some basic stuff


## Idea

The idea was actually to make a 24 strip circle, not a half circle of 12
strips.  Unfortunately the second SPI of the RP-2040 chip ceased working. I
have to check it when I get home with my osci.


## Making of

I've chosen the RP-2040 for the project for two reasons.

* It has two SPIs (on of which is not working ☹️), as I was not able to run all
  the 24 strips on one SPI.

* The SPIs of the RP-2040 are said to deal with 5V quite well and as it turns
  out I have not experienced logic voltage issues – well maybe some small ones.


## Programming

The whole thing is written in Rust. Basically because I wanted to do something
with Embedded Rust.


Feel free to ask questions in the issue tracker.
