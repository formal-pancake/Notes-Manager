# Notes-Manager

Notes-Manager is a [tui](https://docs.rs/tui) app that lets you write and save notes.

![gif showing the application](./assets/demo.gif)

# Installation

### Clone the repository

```sh
> git clone https://github.com/***REMOVED***/Notes-Manager.git
```

### Launch application

```sh
> cargo run --release
```

Or alternatively, download the executable:

https://github.com/***REMOVED***/Notes-Manager/releases

# Controls
 - 'q' - quit the application
 - 'up' and 'down' arrow to navigate the menus
 - 'enter' to select a menu action
 - 'esc' to return to the previous screen or leave writting mode

# Functionalities
 - Write notes
 - Save notes with timestamp
 - Load saved notes on startup

# Coming soon
 - Ability to edit notes
 - Ability to delete notes
 - Ability to scroll the notes menu
 - Ability to choose save location*
> \* current save location is the current directory of the app `./saved-notes.bin`

# Known issues

 - The cursor postion when writting can overflow due to text wrapping (cursor will only go to the next line when pressing enter)
 - Inability to scroll the notes menu