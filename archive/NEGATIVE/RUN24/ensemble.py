"""
Ensemble strategy framework.
Combines multiple strategies via voting or stacking.
"""
import pandas as pd
import numpy as np
from typing import Callable, Optional


def ensemble_vote(strategies: list, df: pd.DataFrame,
                  weights: Optional[list] = None, threshold: float = 0.5):
    """
    Majority/weighted voting ensemble.

    Args:
        strategies: list of strategy functions, each fn(df) -> (entry, exit)
        df: DataFrame with indicators
        weights: optional weights per strategy (default: equal weight)
        threshold: vote threshold for entry (0.5 = majority)

    Returns:
        (entry, exit) boolean Series tuple
    """
    n = len(strategies)
    if weights is None:
        weights = [1.0 / n] * n

    entry_votes = pd.DataFrame(index=df.index)
    exit_votes = pd.DataFrame(index=df.index)

    for i, strat_fn in enumerate(strategies):
        try:
            entry, exit_sig = strat_fn(df)
            entry_votes[f's{i}'] = entry.astype(float) * weights[i]
            exit_votes[f's{i}'] = exit_sig.astype(float) * weights[i]
        except Exception:
            entry_votes[f's{i}'] = 0.0
            exit_votes[f's{i}'] = 0.0

    entry_score = entry_votes.sum(axis=1)
    exit_score = exit_votes.sum(axis=1)

    entry_signal = entry_score >= threshold
    exit_signal = exit_score >= threshold

    return entry_signal, exit_signal


def ensemble_stacking(strategies: list, df: pd.DataFrame,
                      target: pd.Series, train_ratio: float = 0.67,
                      meta_learner: str = 'logistic'):
    """
    Stacking ensemble: use strategy signals as features for a meta-learner.

    Args:
        strategies: list of strategy functions
        df: DataFrame with indicators
        target: binary target (1 = profitable trade, 0 = not)
        train_ratio: fraction for training the meta-learner
        meta_learner: 'logistic' or 'rf'

    Returns:
        (entry, exit) boolean Series tuple
    """
    from sklearn.linear_model import LogisticRegression
    from sklearn.ensemble import RandomForestClassifier

    # Build meta-features from strategy signals
    meta_features = pd.DataFrame(index=df.index)
    exit_signals = {}

    for i, strat_fn in enumerate(strategies):
        try:
            entry, exit_sig = strat_fn(df)
            meta_features[f'entry_{i}'] = entry.astype(float)
            meta_features[f'exit_{i}'] = exit_sig.astype(float)
            exit_signals[i] = exit_sig
        except Exception:
            meta_features[f'entry_{i}'] = 0.0
            meta_features[f'exit_{i}'] = 0.0

    # Align target
    target = target.reindex(df.index).fillna(0)

    # Train/test split
    split = int(len(df) * train_ratio)
    X_train = meta_features.iloc[:split].fillna(0)
    y_train = target.iloc[:split]
    X_test = meta_features.iloc[split:].fillna(0)

    # Fit meta-learner
    if meta_learner == 'logistic':
        model = LogisticRegression(random_state=42, max_iter=1000)
    else:
        model = RandomForestClassifier(n_estimators=50, max_depth=5, random_state=42)

    model.fit(X_train, y_train)

    # Predict on full dataset
    proba = model.predict_proba(meta_features.fillna(0))[:, 1]
    entry_signal = pd.Series(proba > 0.55, index=df.index)

    # Exit: average of component exit signals
    exit_df = pd.DataFrame({i: sig for i, sig in exit_signals.items()})
    exit_signal = exit_df.mean(axis=1) > 0.5

    return entry_signal, exit_signal
