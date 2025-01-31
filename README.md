clubfridge-neo
==============================================================================

__Digital cash register for airsport clubs.__

![Screenshot](docs/screenshot.png)

This project is a reimplementation of the [Clubfridge](https://new.clubfridge.com/)
project, which was started in 2017 to make it easier for airsport clubs to
sell snacks and beverages to their members.

The original project was implemented with a dedicated cloud server, but these
days it is possible to save sales directly into the [Vereinsflieger](https://www.vereinsflieger.de/)
system, which is used by most airsport clubs in Germany.

The application is intended to run on a [Raspberry Pi](https://www.raspberrypi.org),
so ARM cross-compilation compatibility is a requirement for any changes.


Installation
-------------------------------------------------------------------------------

- Install [Raspberry Pi OS](https://www.raspberrypi.org/software/operating-systems/)
  (64-bit) on a fresh SD card using e.g. [Raspberry Pi Imager](https://www.raspberrypi.org/software/)
- Boot the Raspberry Pi and connect it to the internet.
- Adjust the screen rotation, if necessary.
- Run `sudo apt-get update` and `sudo apt-get dist-upgrade` to update the system.
- Download the latest release of the `clubfridge-neo` application from the
  [releases page](https://github.com/Turbo87/clubfridge-neo/releases) and extract
  it to the `/home/pi` directory.
- Run `sudo nano /etc/xdg/labwc/autostart` and comment out the first two lines
  to disable autostart of the `pacmanfm` and `wf-panel-pi` applications.
- Run `nano /home/pi/.config/labwc/autostart` and add 
  `/usr/bin/lwrespawn /home/pi/clubfridge-neo --fullscreen --update-button`
  to the end of the file to start the `clubfridge-neo` application when the
  Raspberry Pi boots up.
- Run `sudo mv /usr/share/icons/PiXflat/cursors/left_ptr /usr/share/icons/PiXflat/cursors/left_ptr.bak`
  and `sudo mv /usr/share/icons/PiXflat/cursors/hand1 /usr/share/icons/PiXflat/cursors/hand1.bak`
  to hide the cursor, which is not needed for this touchscreen application.
- Reboot the Raspberry Pi to start the `clubfridge-neo` application.


License
-------------------------------------------------------------------------------

Licensed under either of

* Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license
  ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.


Contribution
-------------------------------------------------------------------------------

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.