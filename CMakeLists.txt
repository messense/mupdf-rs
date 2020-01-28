# This file is used only for IDE support

cmake_minimum_required(VERSION 3.0.0)
project(mupdf-wrapper VERSION 0.1.0)

include_directories(mupdf-sys mupdf-sys/mupdf/include)
add_library(mupdf-wrapper mupdf-sys/wrapper.c)

set(CPACK_PROJECT_NAME ${PROJECT_NAME})
set(CPACK_PROJECT_VERSION ${PROJECT_VERSION})
include(CPack)
