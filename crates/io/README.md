# IO crate for GraphWalker

Reads and writes json and graphwiz formatted files.

For example; reading the example file [login.json](../../resources/models/login.json) and print it to the terminal:

```
cargo run -- convert resources/models/login.json --format dot | graph-easy
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