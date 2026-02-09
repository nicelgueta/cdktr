import time
import numpy as np
import pandas as pd

def main() -> None:
    np.random.seed(42)

    time.sleep(0.5)

    data = np.random.randn(1000, 5)
    df = pd.DataFrame(data, columns=[f"col_{i}" for i in range(5)])

    time.sleep(0.5)

    df["sum"] = df.sum(axis=1)
    df["mean"] = df.mean(axis=1)
    df["std"] = df.std(axis=1)

    time.sleep(0.5)

    summary = df.describe()
    corr = df.corr()

    print("Summary:")
    print(summary)
    print("\nCorrelation:")
    print(corr)

if __name__ == "__main__":
    main()