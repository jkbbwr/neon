(ns main
  (:gen-class))

(set! *warn-on-reflection* true)

;; Word-frequency: string building, string hashing, and hash-map upserts using a
;; transient map with string keys.
(defn -main []
  (let [n 10000000]
    (loop [i (long 0)
           x (long 42)
           counts (transient {})]
      (if (< i n)
        (let [x (rem (* x 48271) 2147483647)
              w (str "w" (rem x 10000))
              c (long (get counts w 0))]
          (recur (inc i) x (assoc! counts w (inc c))))
        (let [m (persistent! counts)
              distinct-count (count m)
              mx (reduce max 0 (vals m))]
          (println (str "Result: " distinct-count " " n " " mx)))))))

(-main)
