# CelTui - Celestial/Astro Navigation helper using Ratatui

CelTui is a set of simple helpers to be used for sight reductions.

## What?

1. Almanac Data Lookup
   Get GHA and Declination for a Body at given time

2. Sight Reduction Tables
   Get Hc for given LHA /where LHA can be decimal but you must use whole number for the sake of proper sight reduction/
   Declination and Lattitude which again must be whole number - the one closest to your Lattitude - for 43°12' it is 43°, for 49°56' it is 50°.

3. Sight Reduction Calculator
   Compute Hc for a sight

4. Automatic Fix Computation
   Compute a fix from several sights and/or a  running fix.

5. Sight Averaging
   Enter multiple sights and get an average, you must plot and observe linear increase in a short period of time, less than 5 minutes.
   Altitude change is linear, so remove *bad* sights before averaging.

6. Arc To Time Calculator
   Convert Arc To Time, i.e. Longitude at LMT (Local Mean Time) to UT



## Why?

During my study of Celestial Navigation as part of my YMO course. I found my self often making simple arithmetic mistakes, so to be able to focus on the process of sight reductions
rather than arithmetic and digging into PDFs and paper books /the course materials/ - CelTui was born.


## Disclaimer

Real alamanc data and SRT data can differ.
Not to be used for navigation but it should be pretty close /if not better/.


## TODO

Implement stars catalogue to compute star's SHA
