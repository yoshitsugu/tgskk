add_executable(test-candidate-navigation tests/candidates.cpp)
add_test(NAME candidate-navigation COMMAND test-candidate-navigation)
add_executable(test-mode-colors tests/mode_colors.cpp)
add_test(NAME mode-colors COMMAND test-mode-colors)
