# How to contribute to Granian

Thank you for considering contributing to Granian!

## Support questions

Please don't use issues for this. Issues are the designed tool to address bugs and feature requests in Granian itself. Use our [Github Discussions](https://github.com/emmett-framework/granian/discussions) section for questions about using Granian or issues with your own code.

## Reporting issues

Include the following information in your post:

- Describe what you expected to happen.
- If possible, include a [minimal reproducible example](https://stackoverflow.com/help/minimal-reproducible-example) to help us identify the issue. This also helps check that the issue is not with your own code.
- Describe what actually happened. Include the full traceback or dump if there was an exception or panic.
- List your Python and Granian versions, also including, if possible, the OS used and the CPU architecture, and, on async protocols, the `asyncio` loop implementation used. If possible, check if this issue is already fixed in the latest releases or the latest code in the repository.

## Submitting patches

If there is not an open issue for what you want to submit, prefer opening one for discussion before working on a PR. You can work on any issue that doesn't have an open PR linked to it or a maintainer assigned to it. These show up in the sidebar. No need to ask if you can work on an issue that interests you.

Include the following in your patch:

- Use the provided formatting commands to format your code. You can setup the environment to run these commands using the instructions below.
- Include tests if your patch add new features or alter existing behavior. Make sure the test fails without your patch.
- Update any relevant documentation.

### First time setup in your local environment

- Make sure you have a GitHub account.
- Make sure you have an updated version of `git`.
- Configure git with your `username` and `email` if needed.
    
      $ git config --global user.name 'your name'
      $ git config --global user.email 'your email'

- Fork Granian to your GitHub account by clicking the [Fork](https://github.com/emmett-framework/granian/fork) button.
- Clone your fork locally, replacing `your-username` in the command below with your actual username.

      $ git clone https://github.com/your-username/granian
      $ cd granian

- Make sure you have the latest stable version of Rust language installed.
- Make sure you have an updated version of [uv](https://github.com/astral-sh/uv).
- Init the environment and build Granian.

      $ make build-dev

### Start coding

- Create a branch to identify the issue you would like to work on. A good format would be using the issue number followed by a minimal description.

      $ git fetch origin
      $ git checkout -b 123-add-some-feature origin/master

- Using your favorite editor, make your changes, [committing as you go](https://afraid-to-commit.readthedocs.io/en/latest/git/commandlinegit.html#commit-your-changes).
- Use the provided formatters and linters on your changes.

      $ make format
      $ make lint

- Push your commits to your fork on GitHub and [create a pull request](https://docs.github.com/en/github/collaborating-with-issues-and-pull-requests/creating-a-pull-request). Link to the issue being addressed with `closes #123` in the pull request description.

### Running the tests

Run the basic test suite with the provided command.

    $ make build-dev
    $ make test

This runs the tests for the current environment, which is usually sufficient. CI will run the full suite when you submit your pull request.
