;; Binary-trees: allocation, teardown, and pointer-chasing. Single-threaded,
;; plain per-node record allocation left to the GC — no pools or arenas.
(ns main
  (:gen-class))

(set! *warn-on-reflection* true)

(deftype Node [left right])

(defn make [^long depth]
  (if (zero? depth)
    (Node. nil nil)
    (Node. (make (dec depth)) (make (dec depth)))))

(defn check ^long [n]
  (if (nil? n)
    0
    (+ 1
       (check (.-left ^Node n))
       (check (.-right ^Node n)))))

(defn -main []
  (let [max-depth 18
        stretch (make (inc max-depth))
        sc (check stretch)]
    (println (str "stretch tree of depth " (inc max-depth) " check: " sc))
    (let [long-lived (make max-depth)
          total (loop [depth 4
                       total (long sc)]
                  (if (<= depth max-depth)
                    (let [iterations (bit-shift-left 1 (+ (- max-depth depth) 4))
                          sum (loop [i 0
                                     sum (long 0)]
                                (if (< i iterations)
                                  (recur (inc i) (+ sum (check (make depth))))
                                  sum))]
                      (println (str iterations " trees of depth " depth " check: " sum))
                      (recur (+ depth 2) (long (+ total sum))))
                    total))
          ll (check long-lived)]
      (println (str "long lived tree of depth " max-depth " check: " ll))
      (println (str "Result: " (+ total ll))))))

(-main)
