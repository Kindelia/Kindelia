#!/bin/sh
# Needs LaTeX and Pandoc installed
# e.g. brew install pandoc basictex
pandoc --pdf-engine=xelatex WHITEPAPER.md -o whitepaper.pdf
