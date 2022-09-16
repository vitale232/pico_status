from machine import Pin, PWM
import socket
import network
from time import sleep

import paint
from secrets import ssid
import server
from waveshare import LCD_1inch14, BL


def listen_for_retry_click(lcd, a_button, b_button):
    while True:
        if a_button.value() == 0:
            paint.paint_reconnect(lcd, "Trying new server...", "If this fails, reboot!")
            sleep(2)
            return
        if b_button.value() == 0:
            paint.paint_reconnect(
                lcd, "Trying to re-establish server...", "If this fails, reboot!"
            )
            sleep(2)
            return


if __name__ == "__main__":
    pwm = PWM(Pin(BL))
    pwm.freq(1000)
    pwm.duty_u16(32768)  # max 65535

    keyA = Pin(15, Pin.IN, Pin.PULL_UP)
    keyB = Pin(17, Pin.IN, Pin.PULL_UP)

    lcd = LCD_1inch14()
    # color BRG
    paint.paint_boot(lcd, "Waiting for connection...")
    lcd.show()

    sleep(2)

    wait_for_conn_secs = 10

    while True:
        connection = None
        try:
            ip = server.connect(wait_for_conn_secs)
            connection = server.open_socket(ip)
            paint.paint_ready(lcd, "Ready and accepting requests!", ip)

            server.serve(connection, lcd)
        except Exception as exc:
            if connection:
                connection.close()
            print(f"An error occurred: {exc}")
            paint.paint_error(lcd, ssid, wait_for_conn_secs, retry=True)
            listen_for_retry_click(lcd, keyA, keyB)
