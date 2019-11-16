# Polar Send Training

Disclaimer: I am NOT affiliated with [Polar](https://www.polar.com).

polar_send_training is a simple stand-alone tool to send training plans to your Polar watch via USB cable. Once sent, you can start the session from the "Favorites" menu. 

During your training session, your watch will show you which phase you are on, and it will beep / vibrate when the phase ends and a new one begins.

The tool is written in Rust, runs in Windows and Linux and doesn't need to be installed. It has been tested with models M430 and M400.


## Usage

Create your training session by using this [website](https://slysherz.github.io/polar-training-session-tool) and download the corresponding file. 
Once you have the training session file, and your watch is connected to your computer, you have 2 options:

### Option 1

Open polar_send_training and select the file or files you want to send to your watch (this way lets you send multiple ones)

### Option 2

Try to open the training session file and, when asked, choose polar_send_training as the program to use. You can save your choice, so that next time you try to open a training session file (.BPB) your computer will automatically use polar_send_training.

## Thanks
[@cmaion](https://github.com/cmaion) for writting a [Ruby tool](https://github.com/cmaion/polar) to interact with Polar watches. This tool is based on his.