# PacRun

Inspired by tom7's incredible journey to [run all the streets of Pittsburgh](https://www.youtube.com/watch?v=1c8i5SABqwU), this was a attempt to automate some map-matching and make a little game out of running all the streets in... wherever I'm living. The idea was that I could upload a GPX file to this app, then it would show me wich streets I ran and save that information into a database. Using this data, I could automatically query for roads I haven't run yet and see some stats on % of roads left, etc. Using OpenStreetMap data, this seemed pretty feasible, but it's starting to not feel that way and I've decided to give up, at least for now.

Here's what was hard:
- Figuring out what OSM data to use was difficult--what types of roads should I include? All of them? If sidewalks are separate from the road (which is what I really care about), how can I match them correctly?
- Map matching itself was complicated--I tried OSRM and found the results were lackluster. I played around briefly with Google and Mapbox's matching APIs but felt limited by the fact that they weren't open-source and didn't present data in the way that I wanted (e.g. with OSM way IDs and such).
- Everything was just too much work for something that wouldn't have much payoff. It's super easy to import GPX files into a PostGIS database and render a nice map in QGIS, and that got me 90% of what I cared about (just seeing where I had run and where I hadn't)
