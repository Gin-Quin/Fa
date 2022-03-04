
# Fa adds the possibility to break multiple loops in once

let t = 0
for x in 1..99
	for y in 1..99
		t++
		if t > 210 do break 2

t = 0
for x in 1..99
	for y in 1..99
		for z in 1..99
			t++
			if t > 210 do break all
