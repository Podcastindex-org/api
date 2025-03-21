# Podcast Index API (v2)

This repository will hold the core code that runs the API. Parts will be posted here as time permits, and they can be
cleaned and commented. No promise is made that the code will be functional as a "release" since different parts will be
released at different times.

# Setup

Connects to MySql using the values in the .env file (see example). Database schema will be released.

<br><br>

### Work log

- 3/21/25
    - API token hashmap is now global
    - Background thread refresh of the token hashmap


- 3/20/25
    - Move config to .env
    - App state/config pass-around
    - Loading all api tokens into a hashmap for quick lookup