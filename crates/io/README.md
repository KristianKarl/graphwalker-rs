# IO crate for GraphWalker

Reads and writes json and [graphviz](https://graphviz.org/) formatted files.

Example:

```
%> cargo run -- convert resources/models/login.json --format dot 
    Finished dev [unoptimized + debuginfo] target(s) in 0.04s
     Running `target/debug/graphwalker convert resources/models/login.json --format dot`
digraph Login {
  n2 [label="v_LoginPrompted\nid: n2"]
  n3 [label="v_Browse\nid: n3"]
  n1 [label="v_ClientNotRunning\nid: n1"]

  n2 -> n2 [label="e_ToggleRememberMe\nid: e5\nAction: rememberMe=!rememberMe;"]
  n1 -> n2 [label="e_StartClient\nid: e1\nGuard: !rememberMe||!validLogin"]
  n2 -> n1 [label="e_Close\nid: e6"]
  n1 -> n1 [label="e_Init\nid: e0\nAction: validLogin=false;rememberMe=false;"]
  n3 -> n2 [label="e_Logout\nid: e3"]
  n1 -> n3 [label="e_StartClient\nid: e7\nGuard: rememberMe&&validLogin"]
  n2 -> n3 [label="e_ValidPremiumCredentials\nid: e2\nAction: validLogin=true;"]
  n2 -> n2 [label="e_InvalidCredentials\nid: e8\nAction: validLogin=false;"]
  n3 -> n1 [label="e_Exit\nid: e4"]
}
```

## graph-easy

A nice way of visualizing graphviz on the terminal can be done using [graph-easy](https://github.com/ironcamel/Graph-Easy).

Example, reading the example file [login.json](../../resources/models/login.json) and print it to the terminal:

```
%> cargo run -- convert resources/models/login.json --format dot | graph-easy
                                                                         e_InvalidCredentials
      e_Logout                                                           id: e8
      id: e3                                                             Action: validLogin=false;
  +----------------------------------+                                 +------------------------------+
  |                                  v                                 v                              |
  |                                +--------------------------------------------------------------------+   e_ToggleRememberMe
  |                                |                                                                    |   id: e5
  |                                |                          v_LoginPrompted                           |   Action: rememberMe=!rememberMe;
  |                                |                               id: n2                               | ---------------------------------------------+
  |                                |                                                                    |                                              |
  |    +-------------------------- |                                                                    | <--------------------------------------------+
  |    |                           +--------------------------------------------------------------------+
  |    |                             |                                 ^
  |    |                             |                                 | e_StartClient
  |    |                             | e_Close                         | id: e1
  |    |                             | id: e6                          | Guard: !rememberMe||!validLogin
  |    |                             v                                 |
  |    |                           +--------------------------------------------------------------------+   e_Init
  |    |                           |                                                                    |   id: e0
  |    | e_ValidPremiumCredentials |                         v_ClientNotRunning                         |   Action: validLogin=false;rememberMe=false;
  |    | id: e2                    |                               id: n1                               | ---------------------------------------------+
  |    | Action: validLogin=true;  |                                                                    |                                              |
  |    |                           |                                                                    | <--------------------------------------------+
  |    |                           +--------------------------------------------------------------------+
  |    |                             |                                 ^
  |    |                             | e_StartClient                   |
  |    |                             | id: e7                          | e_Exit
  |    |                             | Guard: rememberMe&&validLogin   | id: e4
  |    |                             v                                 |
  |    |                           +--------------------------------+  |
  |    |                           |            v_Browse            |  |
  |    +-------------------------> |             id: n3             | -+
  |                                +--------------------------------+
  |                                  |
  +----------------------------------+
```

Install graph-easy for the above to work. On ubuntu
```
sudo apt install libgraph-easy-perl
```