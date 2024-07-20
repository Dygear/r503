# RP2040 Pico W Pinout & Connection
We start by connecting the Grow r503 (Pro) to the RP2040 Pico W with the following pinout.
![RP2040 Pico W Pinout](https://www.raspberrypi.com/documentation/microcontrollers/images/picow-pinout.svg)

| # | Sensor Wire Color                      | Function |  PIN#  | Pin Name                            |
| - |                  :---:                 |   :---:  |  :---: |                :---:                |
| 1 | $${\color{red}RED \space Wire}$$       | Power    |   36   | $${\color{red}3V3(OUT)}$$           |
| 2 | $${\color{white}WHITE \space Wire}$$   | Ground   |   38   | $${\color{black}GND}$$              |
| 3 | $${\color{yellow}YELLOW \space Wire}$$ | TX (Out) |    7   | $${\color{purple}UART1 \space RX}$$ |
| 4 | $${\color{purple}PURPLE \space Wire}$$ | RX (In)  |    6   | $${\color{purple}UART1 \space TX}$$ |
| 5 | $${\color{blue}BLUE \space Wire}$$     | Wakeup   |    9   | $${\color{lightgreen}GPIO 6}$$      |
| 6 | $${\color{white}WHITE \space Wire}$$   | Touch    |   10   | $${\color{lightgreen}GPIO 7}$$      |

We use UART1, because UART0 can be used by the PICO Probe.
