/// A short eye-rest exercise shown during a break. The set below leans
/// toward what helps most with long PC sessions + farsightedness (hyperopia):
/// near-work causes accommodative fatigue and dry eye, so focus-shifting,
/// blinking, and relaxation exercises are mixed in alongside the classic
/// 20-20-20 look-away.
pub struct Exercise {
    pub title: &'static str,
    pub lines: &'static [&'static str],
}

pub const EXERCISES: &[Exercise] = &[
    Exercise {
        title: "👁  20-20-20 Rule",
        lines: &[
            "Look at something 20 feet away",
            "for 20 seconds.",
        ],
    },
    Exercise {
        title: "🔎  Focus Shifting",
        lines: &[
            "Hold a thumb ~10 inches from your face.",
            "Focus on it, then shift focus to something",
            "far away. Repeat 5–6 times.",
            "(Great for farsighted eyes — trains focus flexibility.)",
        ],
    },
    Exercise {
        title: "😌  Palming",
        lines: &[
            "Rub your palms together to warm them,",
            "then gently cup them over closed eyes.",
            "Breathe slowly and relax.",
        ],
    },
    Exercise {
        title: "👀  Eye Rolling",
        lines: &[
            "Slowly roll your eyes in a full circle,",
            "clockwise, then counter-clockwise.",
            "Keep your head still.",
        ],
    },
    Exercise {
        title: "😉  Blinking Break",
        lines: &[
            "Blink slowly and fully 15–20 times.",
            "Screens cut your blink rate way down —",
            "this rewets your eyes and fights dryness.",
        ],
    },
    Exercise {
        title: "♾️  Figure Eight",
        lines: &[
            "Picture a large figure-8 about 10 feet away.",
            "Trace it slowly with your eyes only,",
            "one direction, then reverse.",
        ],
    },
    Exercise {
        title: "🖐  Near-Far Fingers",
        lines: &[
            "Hold one finger close, another arm's length.",
            "Look back and forth between the two,",
            "10 times, focusing sharply on each.",
        ],
    },
];

pub fn get(index: usize) -> &'static Exercise {
    &EXERCISES[index % EXERCISES.len()]
}

pub fn count() -> usize {
    EXERCISES.len()
}
