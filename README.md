# mtt
![Workflow status](https://github.com/markus-k/mtt/actions/workflows/rust.yml/badge.svg)

mtt – a minimal time tracker written in Rust.

mtt allows you to track the time you are working on projects. It has a simple command line interface for starting and stopping the timer, as well as displaying the time spend.

## Usage

To start the timer, simply run
```
mtt start
```
The timer is now running. Once you're done, you can stop the timer with
```
mtt stop
````
and mtt will display the time you spend since starting it, as well as the total time you have tracked.

You can also show the tracked time at any time with `mtt show`:
```
$ mtt show
Current timer: 8s
Total: 1h 21m 31s
```

To reset the toral tracked time, run
```
mtt reset
```

## License

mtt is licensed under the Apache License 2.0
