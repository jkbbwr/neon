;; N-body: the benchmarks-game gravitational integrator, single-threaded.
;; Faithful port of ../c/main.c — identical constants and operation order.
;; Bodies live in one flat primitive double array, stride 7:
;; [x y z vx vy vz mass] per body, so the hot loop is all aget/aset.
(ns main
  (:gen-class))

(set! *warn-on-reflection* true)

(def ^:const PI 3.141592653589793)
(def ^:const SOLAR-MASS (* (* 4.0 PI) PI))
(def ^:const DAYS-PER-YEAR 365.24)
(def ^:const N-BODIES 5)

(defn initial-bodies ^doubles []
  (double-array
   [;; sun
    0.0 0.0 0.0 0.0 0.0 0.0 SOLAR-MASS
    ;; jupiter
    4.84143144246472090e+00 -1.16032004402742839e+00 -1.03622044471123109e-01
    (* 1.66007664274403694e-03 DAYS-PER-YEAR)
    (* 7.69901118419740425e-03 DAYS-PER-YEAR)
    (* -6.90460016972063023e-05 DAYS-PER-YEAR)
    (* 9.54791938424326609e-04 SOLAR-MASS)
    ;; saturn
    8.34336671824457987e+00 4.12479856412430479e+00 -4.03523417114321381e-01
    (* -2.76742510726862411e-03 DAYS-PER-YEAR)
    (* 4.99852801234917238e-03 DAYS-PER-YEAR)
    (* 2.30417297573763929e-05 DAYS-PER-YEAR)
    (* 2.85885980666130812e-04 SOLAR-MASS)
    ;; uranus
    1.28943695621391310e+01 -1.51111514016986312e+01 -2.23307578892655734e-01
    (* 2.96460137564761618e-03 DAYS-PER-YEAR)
    (* 2.37847173959480950e-03 DAYS-PER-YEAR)
    (* -2.96589568540237556e-05 DAYS-PER-YEAR)
    (* 4.36624404335156298e-05 SOLAR-MASS)
    ;; neptune
    1.53796971148509165e+01 -2.59193146099879641e+01 1.79258772950371181e-01
    (* 2.68067772490389322e-03 DAYS-PER-YEAR)
    (* 1.62824170038242295e-03 DAYS-PER-YEAR)
    (* -9.51592254519715870e-05 DAYS-PER-YEAR)
    (* 5.15138902046611451e-05 SOLAR-MASS)]))

(defn offset-momentum [^doubles b]
  (loop [i 0
         px 0.0
         py 0.0
         pz 0.0]
    (if (< i N-BODIES)
      (let [o (* i 7)
            m (aget b (+ o 6))]
        (recur (inc i)
               (+ px (* (aget b (+ o 3)) m))
               (+ py (* (aget b (+ o 4)) m))
               (+ pz (* (aget b (+ o 5)) m))))
      (do
        (aset b 3 (/ (- px) SOLAR-MASS))
        (aset b 4 (/ (- py) SOLAR-MASS))
        (aset b 5 (/ (- pz) SOLAR-MASS))))))

(defn advance [^doubles b ^double dt]
  (loop [i 0]
    (when (< i N-BODIES)
      (let [oi (* i 7)
            mi (aget b (+ oi 6))]
        (loop [j (inc i)]
          (when (< j N-BODIES)
            (let [oj (* j 7)
                  dx (- (aget b oi) (aget b oj))
                  dy (- (aget b (+ oi 1)) (aget b (+ oj 1)))
                  dz (- (aget b (+ oi 2)) (aget b (+ oj 2)))
                  d2 (+ (* dx dx) (* dy dy) (* dz dz))
                  mag (/ dt (* d2 (Math/sqrt d2)))
                  mj (aget b (+ oj 6))]
              (aset b (+ oi 3) (- (aget b (+ oi 3)) (* dx mj mag)))
              (aset b (+ oi 4) (- (aget b (+ oi 4)) (* dy mj mag)))
              (aset b (+ oi 5) (- (aget b (+ oi 5)) (* dz mj mag)))
              (aset b (+ oj 3) (+ (aget b (+ oj 3)) (* dx mi mag)))
              (aset b (+ oj 4) (+ (aget b (+ oj 4)) (* dy mi mag)))
              (aset b (+ oj 5) (+ (aget b (+ oj 5)) (* dz mi mag))))
            (recur (inc j)))))
      (recur (inc i))))
  (loop [i 0]
    (when (< i N-BODIES)
      (let [o (* i 7)]
        (aset b o (+ (aget b o) (* dt (aget b (+ o 3)))))
        (aset b (+ o 1) (+ (aget b (+ o 1)) (* dt (aget b (+ o 4)))))
        (aset b (+ o 2) (+ (aget b (+ o 2)) (* dt (aget b (+ o 5))))))
      (recur (inc i)))))

(defn energy ^double [^doubles b]
  (loop [i 0
         e 0.0]
    (if (< i N-BODIES)
      (let [oi (* i 7)
            mi (aget b (+ oi 6))
            vx (aget b (+ oi 3))
            vy (aget b (+ oi 4))
            vz (aget b (+ oi 5))
            e (+ e (* 0.5 mi (+ (* vx vx) (* vy vy) (* vz vz))))
            e (double
               (loop [j (inc i)
                      e e]
                 (if (< j N-BODIES)
                   (let [oj (* j 7)
                         dx (- (aget b oi) (aget b oj))
                         dy (- (aget b (+ oi 1)) (aget b (+ oj 1)))
                         dz (- (aget b (+ oi 2)) (aget b (+ oj 2)))]
                     (recur (inc j)
                            (- e (/ (* mi (aget b (+ oj 6)))
                                    (Math/sqrt (+ (* dx dx) (* dy dy) (* dz dz)))))))
                   e)))]
        (recur (inc i) e))
      e)))

(defn fmt9 ^String [^double e]
  (String/format java.util.Locale/ROOT "%.9f" (object-array [e])))

(defn -main []
  (let [n 20000000
        b (initial-bodies)]
    (offset-momentum b)
    (let [before (fmt9 (energy b))]
      (println before)
      (loop [k 0]
        (when (< k n)
          (advance b 0.01)
          (recur (unchecked-inc k))))
      (let [after (fmt9 (energy b))]
        (println after)
        (println (str "Result: " before " " after))))))

(-main)
