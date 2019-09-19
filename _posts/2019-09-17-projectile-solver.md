---
layout: post
title: "Projectile solver"
categories: rust
---

## Hitting a moving target with a ballistic projectile

The games I play come with simple ballistic physics for projectile-based weapons: the projectiles are fired with an initial speed and are only affected by gravity. The resulting trajectory is a ballistic trajectory.

Because gravity is a harsh mistress literally every game 'cheats' this and fakes a much lower gravity constant for projectiles than is used for players. Without this hack projectiles are either pathethic or need a very high velocity to compensate.

In this article I present a method of calculating where to aim your weapon which fires a projectile under simple gravity to hit an arbitrary moving target in 3D space. For a video of the result, scroll down to the bottom.

Before we tackle this problem let's consider a more simple example: hitting a stationary target. Let's simplify even further to only consider the 2D case:

Consider the location from which the projectile is fired at `[0, 0]` and we're trying to hit a target at `[x, y]`. As luck would have it [wikipedia's Projectile motion article](https://en.wikipedia.org/wiki/Projectile_motion#Angle_%7F'%22%60UNIQ--postMath-0000003A-QINU%60%22'%7F_required_to_hit_coordinate_\(x,y\)) has the formula for calculating the angle to shoot at:

![](https://wikimedia.org/api/rest_v1/media/math/render/svg/2c5c375af9d89e403690dceeb6d074eab6ed27fe)

An interesting feature is that this equation has either zero, one or two solutions. Intuitively these correspond with whether the projectile has enough speed to even reach the target and if it's too fast we can either shoot directly at the target or lob it up high and hit it on the way back down.

How do we know which of the two solutions to pick? First if the value under the square root is negative then there are no solutions, the target is too far away. Then the final value is put under `atan` to get the angle. The larger the slope value given to `atan` is, the closer the angle is to 90°. A lower slope results in a lower angle (assuming all positive values). Thus the lower slope is the optimal angle and the higher slope lobs the projectile up high.

Finally we can calculate the time the projectile takes to reach the target. This is independent of gravity and is simply derived from the horizontal distance.

```rust
fn optimal_angle(x: f32, y: f32, v0: f32, g: f32) -> Option<f32> {
	let root = v0 * v0 * v0 * v0 - g * (g * x * x + 2.0 * y * v0 * v0);
	if root < 0.0 {
		return None;
	}
	let root = f32::sqrt(root);
	let angle = f32::atan((v0 * v0 - root) / (g * x));
	Some(angle)
}

fn lob_angle(x: f32, y: f32, v0: f32, g: f32) -> Option<f32> {
	let root = v0 * v0 * v0 * v0 - g * (g * x * x + 2.0 * y * v0 * v0);
	if root < 0.0 {
		return None;
	}
	let root = f32::sqrt(root);
	let angle = f32::atan((v0 * v0 + root) / (g * x));
	Some(angle)
}

fn travel_time(x: f32, angle: f32, v0: f32) -> f32 {
	x / (f32::cos(angle) * v0)
}
```

Everything is better with visualizations! Here's the result of the formulas:

<svg width="800" height="450" viewBox="-50 -600 800 450" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"><g transform="translate(0, -200) scale(1,-1)"><line x0="-1000" x1="2000" y0="0" y1="0" stroke="black" vector-effect="non-scaling-stroke" shape-rendering="crispEdges" /><line y0="-1000" y1="1000" x0="0" x1="0" stroke="black" vector-effect="non-scaling-stroke" shape-rendering="crispEdges" /><circle cx="650" cy="150" r="5" fill="green" /><path d="M0.00 0.00L11.79 29.79L23.58 58.57L35.37 86.36L47.16 113.14L58.95 138.93L70.74 163.72L82.53 187.50L94.32 210.29L106.11 232.07L117.90 252.86L129.69 272.65L141.48 291.43L153.27 309.22L165.06 326.00L176.85 341.79L188.64 356.58L200.43 370.36L212.22 383.15L224.01 394.93L235.81 405.72L247.60 415.51L259.39 424.29L271.18 432.08L282.97 438.86L294.76 444.65L306.55 449.44L318.34 453.22L330.13 456.01L341.92 457.79L353.71 458.58L365.50 458.37L377.29 457.15L389.08 454.94L400.87 451.72L412.66 447.51L424.45 442.30L436.24 436.08L448.03 428.87L459.82 420.65L471.61 411.44L483.40 401.22L495.19 390.01L506.98 377.80L518.77 364.58L530.56 350.37L542.35 335.15L554.14 318.94L565.93 301.73L577.72 283.51L589.51 264.30L601.30 244.08L613.09 222.87L624.88 200.66L636.67 177.44L648.46 153.23L650.00 150.00" fill="none" stroke="red" style="opacity: 0.5;" id="B_s1" /><path d="M0.00 0.00L26.86 17.80L53.72 34.60L80.58 50.40L107.44 65.19L134.30 78.99L161.16 91.79L188.01 103.59L214.87 114.39L241.73 124.19L268.59 132.98L295.45 140.78L322.31 147.58L349.17 153.38L376.03 158.18L402.89 161.98L429.75 164.77L456.61 166.57L483.47 167.37L510.33 167.17L537.18 165.97L564.04 163.77L590.90 160.56L617.76 156.36L644.62 151.16L650.00 150.00" fill="none" stroke="blue" style="opacity: 0.5;" id="B_s2" /><circle r="5" fill="red"><animateMotion id="B_m" dur="2.7565143" fill="freeze" begin="0s;B_m.end+1s" calcMode="linear"><mpath xlink:href="#B_s1"></mpath></animateMotion></circle><circle r="5" fill="blue"><animateMotion dur="1.2100123" fill="freeze" begin="0s;B_m.end+1s" calcMode="linear"><mpath xlink:href="#B_s2"></mpath></animateMotion></circle></g></svg>

Legend:
* The projectile is fired with `v0 = 650 u/s`, `g = 400 u/s²`.
* Green is the target at position `[650, 150] u`.
* Blue is the optimal path aimed at `34.27°` and reaches the target in `1.21 s`.
* Red lobs the projectile at `68.73°` and reaches the target in `2.76 s`.

So far so good, we can hit stationary target in 2D. In fact, this also solves the problem of hitting a stationary target in 3D with a clever framing of the problem:

In this simplified world where projectiles are only affected by gravity, consider the vertical plane passing through the player and the target. The projectile fired by the player travels in this plane! Thus we can construct a 2D setup which when solved results in an angle aimed at the target and pitched with the resulting angle from the 2D solution.

(still working on 3D animations!)

So how do we hit a moving target? If we aimed at the target as if he stood still, when the projectile reaches the target at t<sub>hit</sub>. However at this point the target would have moved some distance away. Now let t<sub>guess</sub> start at `0 s` and increment in steps of, say, `0.01 s`. Predict the target's position at t<sub>guess</sub> and fire a projectile at this position. Compare the projectile's travel time t<sub>hit</sub> with our chosen t<sub>guess</sub>. Initially t<sub>guess</sub> is less than t<sub>hit</sub> but as t<sub>guess</sub> increases these values converge (or not in case the target is not reachable). When t<sub>guess</sub> first gets larger (but still close to) t<sub>hit</sub> we have a fairly accurate estimation of when and where a projectile, when fired at this position, will hit the target.

All of this hinges on a good prediction of the target. In the following demo I will use simple linear extrapolation, but quadratic extrapolation to simulate gravity works just as well. This is a very crude approximation of the future and gets less reliable the further in the future is predicted. We can put an arbitrary cap where the accuracy of this extrapolation becomes too inaccurate. For the purpose of this demo it doesn't matter as we'll make the target move predictably.

The algorithm in code:

```rust
struct Target {
	position: [f32; 2],
	velocity: [f32; 2],
	gravity: f32,
}
impl Target {
	fn predict(&self, time: f32) -> [f32; 2] {
		let x = self.position[0] + self.velocity[0] * time;
		let y = self.position[1] + self.velocity[1] * time - self.gravity * time * time * 0.5;
		[x, y]
	}
}

struct Weapon {
	speed: f32,
	gravity: f32,
}

const MAX_TIME: f32 = 5.5;
const TIME_STEP: f32 = 0.01;

struct Solver {
	target: Target,
	weapon: Weapon,
}
impl Solver {
	fn solve(&self) -> Option<f32> {
		let mut target_time = 0.0;
		while target_time < MAX_TIME {
			let target_pos = self.target.predict(target_time);
			let sol_angle = self.angle(target_pos)?;
			let sol_time = travel_time(target_pos[0], sol_angle, self.weapon.speed);
			if sol_time < target_time {
				return Some(sol_angle);
			}
			target_time += TIME_STEP;
		}
		None
	}
	fn angle(&self, pos: [f32; 2]) -> Option<f32> {
		optimal_angle(pos[0], pos[1], self.weapon.speed, self.weapon.gravity)
	}
}
```

And visualized:

<svg width="800" height="450" viewBox="-50 -600 800 450" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"><g transform="translate(0, -200) scale(1,-1)"><line x0="-1000" x1="2000" y0="0" y1="0" stroke="black" vector-effect="non-scaling-stroke" shape-rendering="crispEdges" /><line y0="-1000" y1="1000" x0="0" x1="0" stroke="black" vector-effect="non-scaling-stroke" shape-rendering="crispEdges" /><path d="M450.00 0.00L455.00 4.94L460.00 9.75L465.00 14.44L470.00 19.00L475.00 23.44L480.00 27.75L485.00 31.94L490.00 36.00L495.00 39.94L500.00 43.75L505.00 47.44L510.00 51.00L515.00 54.44L520.00 57.75L525.00 60.94L530.00 64.00L535.00 66.94L540.00 69.75L545.00 72.44L550.00 75.00L555.00 77.44L560.00 79.75L565.00 81.94L570.00 84.00L575.00 85.94L580.00 87.75L585.00 89.44L590.00 91.00L595.00 92.44L600.00 93.75L605.00 94.94L610.00 96.00L615.00 96.94L620.00 97.75L625.00 98.44L630.00 99.00L635.00 99.44L640.00 99.75L645.00 99.94L650.00 100.00L655.00 99.94L660.00 99.75L665.00 99.44L670.00 99.00L675.00 98.44L680.00 97.75L685.00 96.94L690.00 96.00L695.00 94.94L700.00 93.75L705.00 92.44L710.00 91.00L715.00 89.44L720.00 87.75L724.96 85.95" fill="none" stroke="green" style="opacity: 0.5;" id="A_s0" /><path d="M0.00 0.00L14.19 25.93L28.37 50.87L42.56 74.80L56.74 97.74L70.93 119.67L85.11 140.60L99.30 160.54L113.49 179.47L127.67 197.41L141.86 214.34L156.04 230.28L170.23 245.21L184.42 259.14L198.60 272.08L212.79 284.01L226.97 294.95L241.16 304.88L255.34 313.81L269.53 321.75L283.72 328.68L297.90 334.62L312.09 339.55L326.27 343.49L340.46 346.42L354.64 348.35L368.83 349.29L383.02 349.22L397.20 348.16L411.39 346.09L425.57 343.02L439.76 338.96L453.95 333.89L468.13 327.83L482.32 320.76L496.50 312.69L510.69 303.63L524.87 293.56L539.06 282.50L553.25 270.43L567.43 257.37L581.62 243.30L595.80 228.23L609.99 212.17L624.17 195.10L638.36 177.04L652.55 157.97L666.73 137.90L680.92 116.84L695.00 94.94" fill="none" stroke="red" style="opacity: 0.5;" id="A_s1" /><path d="M0.00 0.00L26.43 13.69L52.87 26.37L79.30 38.06L105.74 48.74L132.17 58.43L158.60 67.12L185.04 74.80L211.47 81.49L237.91 87.17L264.34 91.86L290.77 95.55L317.21 98.23L343.64 99.92L370.08 100.61L396.51 100.29L422.94 98.98L449.38 96.66L475.81 93.35L502.25 89.04L528.68 83.72L555.00 77.44" fill="none" stroke="blue" style="opacity: 0.5;" id="A_s2" /><circle r="5" fill="green"><animateMotion dur="2.7496343" fill="freeze" begin="0s;A_m.end+1s" calcMode="linear"><mpath xlink:href="#A_s0"></mpath></animateMotion></circle><circle r="5" fill="red"><animateMotion id="A_m" dur="2.4496343" fill="freeze" begin="0s;A_m.end+1s" calcMode="linear"><mpath xlink:href="#A_s1"></mpath></animateMotion></circle><circle r="5" fill="blue"><animateMotion dur="1.0497859" fill="freeze" begin="0s;A_m.end+1s" calcMode="linear"><mpath xlink:href="#A_s2"></mpath></animateMotion></circle></g></svg>

Legend:
* The projectile is fired with `v0 = 600 u/s`, `g = 400 u/s²`.
* Green is the target at position `[450, 0] u`, with velocity `[100, 100] u/s` and gravity of `20 u/s²`.
* Blue is the optimal path aimed at `28.22°` and reaches the target in `1.05 s`.
* Red lobs the projectile at `61.78°` and reaches the target in `2.45 s`.

The above animation shows the solver in 2D action but is trivially extended to 3D. Plug in an appropriate target position predictor and construct the 2D setup before solving.

Just to show that the target predictor isn't restricted to linear trajectory, in the following animation the target is making a circular motion and the solver has no issues hitting the target:

<svg width="800" height="450" viewBox="-50 -600 800 450" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"><g transform="translate(0, -200) scale(1,-1)"><line x0="-1000" x1="2000" y0="0" y1="0" stroke="black" vector-effect="non-scaling-stroke" shape-rendering="crispEdges" /><line y0="-1000" y1="1000" x0="0" x1="0" stroke="black" vector-effect="non-scaling-stroke" shape-rendering="crispEdges" /><path d="M750.00 50.00L748.00 60.98L745.01 71.87L741.03 82.55L736.11 92.94L730.26 102.94L723.53 112.46L715.98 121.42L707.67 129.74L698.66 137.33L689.03 144.15L678.86 150.12L668.24 155.20L657.25 159.36L646.00 162.54L634.57 164.75L623.08 165.96L611.62 166.17L600.28 165.38L589.17 163.63L578.39 160.93L568.02 157.32L558.15 152.85L548.87 147.57L540.26 141.55L532.39 134.85L525.31 127.55L519.09 119.74L513.78 111.50L509.40 102.92L506.00 94.11L503.59 85.16L502.17 76.16L501.75 67.23L502.32 58.45L503.85 49.92L506.32 41.75L509.69 34.02L513.90 26.81L518.91 20.22L524.64 14.32L531.02 9.17L537.97 4.84L545.42 1.38L553.27 -1.16L561.42 -2.75L569.78 -3.37L578.26 -2.99L586.75 -1.62L595.15 0.75L603.37 4.11L611.30 8.42L618.85 13.65L625.94 19.77L632.47 26.72L638.37 34.45L643.56 42.87L647.97 51.93L651.55 61.54L654.25 71.61L656.02 82.06L656.83 92.78L656.65 103.69L655.49 114.68L653.32 125.65L651.24 133.19" fill="none" stroke="green" style="opacity: 0.5;" id="C_s0" /><path d="M0.00 0.00L11.14 30.03L22.28 59.06L33.42 87.09L44.56 114.13L55.70 140.16L66.83 165.19L77.97 189.22L89.11 212.25L100.25 234.28L111.39 255.31L122.53 275.35L133.67 294.38L144.81 312.41L155.95 329.44L167.09 345.47L178.23 360.50L189.36 374.53L200.50 387.57L211.64 399.60L222.78 410.63L233.92 420.66L245.06 429.69L256.20 437.72L267.34 444.76L278.48 450.79L289.62 455.82L300.76 459.85L311.90 462.88L323.03 464.91L334.17 465.94L345.31 465.98L356.45 465.01L367.59 463.04L378.73 460.07L389.87 456.10L401.01 451.13L412.15 445.16L423.29 438.20L434.43 430.23L445.56 421.26L456.70 411.29L467.84 400.32L478.98 388.35L490.12 375.38L501.26 361.42L512.40 346.45L523.54 330.48L534.68 313.51L545.82 295.54L556.96 276.57L568.09 256.60L579.23 235.64L590.37 213.67L601.51 190.70L612.65 166.73L623.79 141.76L634.93 115.79L646.07 88.82L653.78 69.56" fill="none" stroke="red" style="opacity: 0.5;" id="C_s1" /><path d="M0.00 0.00L27.07 17.49L54.13 33.98L81.20 49.47L108.27 63.96L135.34 77.45L162.40 89.94L189.47 101.43L216.54 111.92L243.60 121.40L270.67 129.89L297.74 137.38L324.81 143.87L351.87 149.36L378.94 153.85L406.01 157.34L433.07 159.83L460.14 161.32L487.21 161.81L514.28 161.30L541.34 159.79L568.02 157.32" fill="none" stroke="blue" style="opacity: 0.5;" id="C_s2" /><circle r="5" fill="green"><animateMotion dur="3.2346222" fill="freeze" begin="0s;C_m.end+1s" calcMode="linear"><mpath xlink:href="#C_s0"></mpath></animateMotion></circle><circle r="5" fill="red"><animateMotion id="C_m" dur="2.9346223" fill="freeze" begin="0s;C_m.end+1s" calcMode="linear"><mpath xlink:href="#C_s1"></mpath></animateMotion></circle><circle r="5" fill="blue"><animateMotion dur="1.04927" fill="freeze" begin="0s;C_m.end+1s" calcMode="linear"><mpath xlink:href="#C_s2"></mpath></animateMotion></circle></g></svg>

If you're interested in reading more about this topic the following articles may be of interest:

* https://www.forrestthewoods.com/blog/solving_ballistic_trajectories/

  This person tries to analytically solve the movements of the projectile and the target moving in a linear motion. For my purposes it is important that the target predictor can be chosen freely and thus his analytic solution does not apply here.

* https://www.hindawi.com/journals/ijcgt/2014/463489/

  This article is more advanced as it incorporates linear drag into the solver as well as solves for more than just the angle to shoot at to hit the target. The article only tries to hit stationary targets but briefly hints at an iterative approach to use the same solution to hit a moving target. It is that solution that I present here. Plug in his solutions for hitting a stationary target with fixed initial speed projectile and now you can hit moving targets in 3D!

The animations were made with custom code and rendering to svg with [format_xml](https://github.com/CasualX/format_xml). My personal goal with this article is to experiment with quickly and simply visualizing algorithms and using Rust + SVG as my canvas.

This isn't something new, but I wanted to write this down for future reference. Years ago I made a video demonstrating the results of my efforts in [Team Fortress 2](https://en.wikipedia.org/wiki/Team_Fortress_2):

[![](https://img.youtube.com/vi/MHZ35b_q_Gc/0.jpg)](https://www.youtube.com/watch?v=MHZ35b_q_Gc)

Discuss on [/r/programming](https://old.reddit.com/r/programming/comments/d5g2d8/projectile_solver_hitting_a_moving_target_with_a/) and [UnKnoWnCheaTs](https://www.unknowncheats.me/forum/general-programming-and-reversing/355088-projectile-aimbot-solver.html). Source code available on [github](https://github.com/CasualX/ProjectileSolverDemo).

Edited a few times since release to add references and fix incorrect timing in the animated objects.

Thanks for reading!
