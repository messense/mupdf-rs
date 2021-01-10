FROM rustembedded/cross:aarch64-linux-android-0.2.1

RUN apt-get update && \
  apt-get install -y lsb-release wget software-properties-common apt-transport-https libc6-dev-i386 && \
  bash -c "$(wget -O - https://apt.llvm.org/llvm.sh)"
