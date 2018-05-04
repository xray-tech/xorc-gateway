def build = fileLoader.fromGit('build', 'git@bitbucket.org:360dialog-berlin/jenkins-scripts.git', 'master', 'git', '')

node('master') {
  build.start { ->
    stage 'Checkout'
    checkout scm

    stage 'Submodule update'
    sh "git submodule update --init"

    stage "Run the tests"
    sh "cargo test"

    stage "Create the release binary"
    sh "cargo build --release"
  }
}
