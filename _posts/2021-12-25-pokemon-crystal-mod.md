---
layout: post
title: "Modding Pokémon Crystal for Fun"
categories: pokemon,rom hack
---

When I was young I loved Pokémon, specifically the Pokémon Game Boy games generations I and II. I played Pokémon Yellow and Gold but grew out of the series and never really got into the next generations.

I occasionally go back and play these old games in Emulator (yay Nostalgia), however these games did not age well. Too much tedium combined with restrictions I don't like. The games are also very easy (they're kids' games after all).

There are many ways to play these old Pokémon games: Nuzlocke, Randomizer, but I prefer a simple Casual playthrough. I like to train as many Pokémon as possible. However, there are just not enough Trainers around to effectively train many Pokémon, and fighting Wild Pokémon is boring.

So what if I could modify these Pokémon games to allow me to rebattle already beaten Trainers?

I have no experience modding Game Boy games and it seemed like a fun project. Lucky for me Pokémon Crystal is fully disassembled and annotated: https://github.com/pret/pokecrystal

## Analysis

Here begins my journey to figure out how to modify Pokémon Crystal to allow rebattling arbitrary Trainers.

Without prior knowledge, I start looking for how Pokémon Trainers are implemented. I quickly find the maps section, here [`AzaleaGym.asm`](https://github.com/pret/pokecrystal/blob/master/maps/AzaleaGym.asm) (I snipped out a bunch of irrelevant data):

```
;...

TrainerBugCatcherBenny:
	trainer BUG_CATCHER, BUG_CATCHER_BENNY, EVENT_BEAT_BUG_CATCHER_BENNY, BugCatcherBennySeenText, BugCatcherBennyBeatenText, 0, .AfterScript

;...

AzaleaGym_MapEvents:
;...
	object_event  5,  7, SPRITE_BUGSY, SPRITEMOVEDATA_SPINRANDOM_SLOW, 0, 0, -1, -1, PAL_NPC_GREEN, OBJECTTYPE_SCRIPT, 0, AzaleaGymBugsyScript, -1
	object_event  5,  3, SPRITE_BUG_CATCHER, SPRITEMOVEDATA_SPINRANDOM_FAST, 0, 0, -1, -1, PAL_NPC_BROWN, OBJECTTYPE_TRAINER, 2, TrainerBugCatcherBenny, -1
```

This shows a bit how the Pokémon games are constructed. The NPCs are a generic `object_event` however it looks like trainers are annotated with `OBJECTTYPE_TRAINER` while the Gym Leader Bugsy is an `OBJECTTYPE_SCRIPT`.

It also reveals that this game has a complex custom scripting engine and there's not just an NPC object, but all interactive elements are simple objects with scripts attached to them.

Each trainer (and all other events) have an `EVENT_*` flag associated with them which can be toggled to indicate the event was triggered. For trainers this is used to track if you've beaten them.

However when looking for references to `OBJECTTYPE_TRAINER` (ignoring the maps) leads to a piece of code in [`trainers.asm`](https://github.com/pret/pokecrystal/blob/master/home/trainers.asm) checking if you walk in sight of an unbeaten trainer who will stop and challenge you: [`CheckTrainerBattle`](https://github.com/pret/pokecrystal/blob/master/home/trainers.asm#L13). This code loops over all map objects and checks if the object:

1. Has a sprite,
2. Is a trainer (checking `OBJECTTYPE_TRAINER`),
3. Is visible on the map,
4. Is facing the player,
5. Within sight range,
6. And hasn't already been beaten.

The first idea pops up in my head: simply patch this last check to ignore whether you've already beaten the trainer. Unfortunately this won't work as the game will initiate a rebattle the moment you exit the battle since the trainer is still facing you.

For now, let's focus on understanding how the event tracking system works:

That last step 6. invokes a function [`EventFlagAction`](https://github.com/pret/pokecrystal/blob/master/home/flag.asm#L27) with arguments `b` = `CHECK_FLAG` and `de` = the event flag index. Let's keep this in the back of our minds.

Back in [`trainers.asm`](https://github.com/pret/pokecrystal/blob/master/home/trainers.asm) looking a bit further, we see this interesting label [`TalkToTrainer`](https://github.com/pret/pokecrystal/blob/master/home/trainers.asm#L103). It would be nice if I could modify the game to only initiate trainer rebattle by talking to them again.

Something interesting about the line containing `TalkToTrainer::`: It has this weird double colon at the end, and no code in this file seems to refer to it. Could this mean this is a symbol visible to other code outside this file? Let's see if there are any interesting references.

It matches in 2 other locations: [overworld/events.asm](https://github.com/pret/pokecrystal/blob/master/engine/overworld/events.asm#L614) and [events/trainer_scripts.asm](https://github.com/pret/pokecrystal/blob/master/engine/events/trainer_scripts.asm):

```

PlayerEventScriptPointers:
; entries correspond to PLAYEREVENT_* constants
	table_width 3, PlayerEventScriptPointers
;...
	dba TalkToTrainerScript     ; PLAYEREVENT_TALKTOTRAINER

;...

.trainer
	call TalkToTrainer
	ld a, PLAYEREVENT_TALKTOTRAINER
	scf
	ret
```

Now we're cooking!

It appears talking to `OBJECTTYPE_TRAINER` objects invokes the `.trainer` code, which invokes `TalkToTrainer`. What it does is not important. It returns some `PLAYEREVENT_TALKTOTRAINER` which probably ends up executing some script named `TalkToTrainerScript`:

```
TalkToTrainerScript::
	faceplayer
	trainerflagaction CHECK_FLAG
	iftrue AlreadyBeatenTrainerScript
	loadtemptrainer
	encountermusic
	sjump StartBattleWithMapTrainerScript
```

This script does some obvious things:

1. Make the trainer NPC you talked to face the player
2. Check if you've already beaten the trainer
3. If true, continue with the `AlreadyBeatenTrainerScript`
4. Otherwise do stuff that initiates the trainer battle!

Conceptually, modifying this script to ignore whether you've already beaten the Trainer and initiate a battle always is pretty simple: Delete the `iftrue ..` bit.

## Modifying the ROM

Patching a binary file is not trivial. Even if you know where you want your modifications to be, you typically cannot insert or remove any bytes because doing so would shift over the rest of the file.

Binary files often contain references (offsets) to other locations. When inserting or deleting bytes these offsets become invalid, corrupting the file.

This means that we must modify the existing ROM without inserting or deleting any bytes. We can only modify existing bytes, which puts a lot of restrictions on what we can do.

However, you can typically replace any code with 'no-ops' which effectively do nothing. As long as you only need to remove code, you can get away with this.

Finally, we'll need to figure out where exactly in the ROM this `TalkToTrainerScript` is located. Let's start with that. I looked at where and how these symbols `faceplayer`, `trainerflagaction`, `CHECK_FLAG` and `iftrue` are defined:

```
	const iffalse_command ; $08
iffalse: MACRO
	db iffalse_command
	dw \1 ; pointer
ENDM

	const iftrue_command ; $09
iftrue: MACRO
	db iftrue_command
	dw \1 ; pointer
ENDM

	const trainerflagaction_command ; $63
trainerflagaction: MACRO
	db trainerflagaction_command
	db \1 ; action
ENDM

	const faceplayer_command ; $6b
faceplayer: MACRO
	db faceplayer_command
ENDM

; FlagAction arguments (see home/flag.asm)
	const_def
	const RESET_FLAG
	const SET_FLAG
	const CHECK_FLAG
```

From this, we can figure out the first bytes of the `TalkToTrainerScript`: `6b 63 02 09`. Open the Pokémon ROM in [HxD](https://en.wikipedia.org/wiki/HxD) and search for these hex values for which it finds exactly 1 match! Success!

After some thinking I would like to change the `TalkToTrainerScript` into:

```
TalkToTrainerScript::
	faceplayer
	trainerflagaction CLEAR_FLAG
	iffalse AlreadyBeatenTrainerScript
	loadtemptrainer
	encountermusic
	sjump StartBattleWithMapTrainerScript
```

Instead of checking the event flag, which tracks whether you've beaten a trainer, it gets cleared. This has a side-effect of always causing `iftrue` so let's change that one into `iffalse`. After you defeat the trainer, their flag will get set again; no worries that the game might get soft-locked.

Looking back at the values, this means changing `6b 63 02 09` into `6b 63 00 08`. After making the changes with [HxD](https://en.wikipedia.org/wiki/HxD) I load up the ROM and try it out:

<video controls>
	<source src="https://casualhacks.net/blog/images/patchedrom.mp4" type="video/mp4">
</video>

## Live Demo

This being the web there's no reason why I can't include a script that applies the modification to your ROM.

Since I'm here, I included some other 'improvement' patches.

Provide an appropriate ROM file and choose the mods you'd like to enable, you can always come back and disable any mods. Click 'Patch ROM' to download the modified ROM file. Enjoy!

<label>Pokémon Gold/Silver/Crystal ROM: <input type="file" id="rominput" accept=".gbc"></label>

<label><input type="checkbox" id="rebattle-trainers"> Rebattle trainers</label> (Talk to already beaten trainer to fight them again.)  
<label><input type="checkbox" id="infinite-tms"> Infinite TMs</label> (TMs are not consumed when used.)  
<label><input type="checkbox" id="forget-hms"> Forgettable HMs</label> (HMs can be forgotten by learning new moves.)  
<label><input type="checkbox" id="boost-exp"> Boosted Exp.</label> (Pokémon always gain boosted experience points.)  
<label><input type="checkbox" id="reset-rtc"> Passwordless RTC Reset</label> (Change the real-time clock with any password.)  

<button onclick="patchROM()">Patch ROM</button>

<script>
function checkPattern(bytes, offset, pattern) {
	if (offset < 0 || offset > bytes.length - pattern.length) {
		return false;
	}
	for (let j = 0; j < pattern.length; j += 1) {
		if (pattern[j] == 0xff) {
			continue;
		}
		if (bytes[offset + j] != pattern[j]) {
			return false;
		}
	}
	return true;
}
function findPattern(bytes, pattern) {
	if (pattern.length <= 0) {
		return false;
	}
	let p0 = pattern[0];
	for (let i = 0; i <= bytes.length - pattern.length; i += 1) {
		if (bytes[i] == p0 && checkPattern(bytes, i, pattern)) {
			return i;
		}
	}
	return false;
}
function patchRebattleTrainers(rom, action) {
	let offset = findPattern(rom, [0x00, 0xFE, 0x1E]);
	console.log("RebattleTrainers", action, offset);
	if (offset === false) {
		return false;
	}
	if (action != 'CHECK') {
		if (action == 'PATCH') {
			rom[offset - 12] = 0x00;
			rom[offset - 11] = 0x08;
		}
		else {
			rom[offset - 12] = 0x02;
			rom[offset - 11] = 0x09;
		}
	}
	return true;
}
function patchInfiniteTMs(rom, action) {
	let offset = findPattern(rom, [0x06, 0x00, 0x4F, 0x09, 0x7E, 0xA7, 0xff, 0x3D, 0x77, 0xC0]);
	console.log("InfiniteTMs", action, offset);
	if (offset === false) {
		return false;
	}
	if (action != 'CHECK') {
		let byte = action == 'PATCH' ? 0xC9 : 0xC8;
		rom[offset + 6] = byte;
	}
	return true;
}
function patchForgetHMs(rom, action) {
	let offset = findPattern(rom, [0xC1, 0xD1, 0x7A, 0x38, 0xff, 0xE1, 0x09, 0xA7, 0xC9]);
	console.log("ForgetHMs", action, offset);
	if (offset === false) {
		return false;
	}
	if (action != 'CHECK') {
		let byte = action == 'PATCH' ? 0x00 : 0x04;
		rom[offset + 4] = byte;
	}
	return true;
}
function patchBoostExp(rom, action) {
	let offset = findPattern(rom, [0xBE, 0x3E, 0x00, 0x28]);
	console.log("BoostExp", action, offset);
	if (offset === false) {
		return false;
	}
	if (action != 'CHECK') {
		let byte = action == 'PATCH' ? 0x00 : 0x05;
		rom[offset + 4] = byte;
	}
	return true;
}
function patchResetRTC(rom, action) {
	let offset = findPattern(rom, [0xFE, 0x01, 0xC8, 0xCD, 0xff, 0xff, 0xff, 0x14]);
	console.log("ResetRTC", action, offset);
	if (offset === false) {
		return false;
	}
	if (action != 'CHECK') {
		let byte = action == 'PATCH' ? 0x30 : 0x38;
		rom[offset + 6] = byte;
	}
	return true;
}
function downloadBytes(data, fileName, mimeType) {
	let blob = new Blob([data], {
		type: mimeType
	});
	let url = window.URL.createObjectURL(blob);
	downloadURL(url, fileName);
	setTimeout(function() {
		return window.URL.revokeObjectURL(url);
	}, 1000);
}
function downloadURL(data, fileName) {
	let a;
	a = document.createElement('a');
	a.href = data;
	a.download = fileName;
	document.body.appendChild(a);
	a.style = 'display: none';
	a.click();
	a.remove();
}
function patchROM() {
	let inputROM = document.getElementById('rominput');
	let inputRebattleTrainers = document.getElementById('rebattle-trainers');
	let inputInfiniteTMs = document.getElementById('infinite-tms');
	let inputForgetHMs = document.getElementById('forget-hms');
	let inputBoostExp = document.getElementById('boost-exp');
	let inputResetRTC = document.getElementById('reset-rtc');
	let buttonPatchRom = document.getElementById('patch-rom');

	if (inputROM.files.length != 1) {
		alert("Please provide a Pokémon Generation II ROM file.");
		return;
	}
	let file = inputROM.files[0];
	let reader = new FileReader();
	reader.onload = () => {
		let bytes = new Uint8Array(reader.result);
		let success = true;

		success &= patchRebattleTrainers(bytes, inputRebattleTrainers.checked ? 'PATCH' : 'CLEAR');
		success &= patchInfiniteTMs(bytes, inputInfiniteTMs.checked ? 'PATCH' : 'CLEAR' );
		success &= patchForgetHMs(bytes, inputForgetHMs.checked ? 'PATCH' : 'CLEAR');
		success &= patchBoostExp(bytes, inputBoostExp.checked ? 'PATCH' : 'CLEAR');
		success &= patchResetRTC(bytes, inputResetRTC.checked ? 'PATCH' : 'CLEAR');

		if (!success) {
			alert("The patterns were not found, could not apply patch!");
		}
		else {
			downloadBytes(bytes, file.name, "application/octet-stream");
		}
	};
	reader.readAsArrayBuffer(file);
}
</script>

## Afterword

That was a fun project! Casual playthroughs of these old games are more enjoyable for my playstyle.
