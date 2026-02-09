"""
Type stubs for the cdktr Python module.

This module provides Python bindings for the cdktr (Cloud DevKit Task Runner) API.
"""

from typing import Optional

class Result:
    """
    Result returned from Principal API calls.

    Attributes:
        success: Whether the operation succeeded.
        error: Optional error message if the operation failed.
        payload: Optional payload data from successful operations.
    """

    success: bool
    error: Optional[str]
    payload: Optional[ str | dict | list | float | int]

    def __init__(
        self,
        success: bool,
        error: Optional[str] = None,
        payload: Optional[ str | dict | list | float | int] = None
    ) -> None:
        """
        Create a new Result.

        Args:
            success: Whether the operation succeeded.
            error: Optional error message.
            payload: Optional payload data.
        """
        ...

    def __repr__(self) -> str:
        """Return a string representation of the Result."""
        ...

class Principal:
    """
    Python wrapper for the Principal API client.

    The Principal is the main orchestrator in cdktr that manages workflows,
    agents, and task execution.
    """

    def __init__(self, host: str = "0.0.0.0", port: int = 5561) -> None:
        """
        Create a new Principal API client.

        Args:
            host: The hostname of the principal server. Defaults to "0.0.0.0".
            port: The port of the principal server. Defaults to 5561.
        """
        ...

    def ping(self) -> Result:
        """
        Ping the principal to check if it's online.

        Returns:
            Result indicating whether the principal is reachable.
        """
        ...

    def list_workflows(self) -> Result:
        """
        List all workflows in the workflow store.

        Returns:
            Result with payload containing JSON array of workflows.
        """
        ...

    def run_workflow(self, workflow_id: str) -> Result:
        """
        Run a workflow by ID.

        Args:
            workflow_id: The ID of the workflow to run.

        Returns:
            Result indicating whether the workflow was started successfully.
        """
        ...

    def query_logs(
        self,
        start_timestamp_ms: Optional[int] = None,
        end_timestamp_ms: Optional[int] = None,
        workflow_id: Optional[str] = None,
        workflow_instance_id: Optional[str] = None,
        verbose: bool = False
    ) -> Result:
        """
        Query logs from the database.

        Args:
            start_timestamp_ms: Optional start timestamp in milliseconds.
            end_timestamp_ms: Optional end timestamp in milliseconds.
            workflow_id: Optional workflow ID to filter logs.
            workflow_instance_id: Optional workflow instance ID to filter logs.
            verbose: Whether to include verbose log details. Defaults to False.

        Returns:
            Result with payload containing JSON array of log entries.
        """
        ...

    def get_recent_workflow_statuses(self) -> Result:
        """
        Get recent workflow statuses (last 10 workflows).

        Returns:
            Result with payload containing JSON array of workflow statuses.
        """
        ...

    def get_registered_agents(self) -> Result:
        """
        Get list of all registered agents.

        Returns:
            Result with payload containing JSON array of registered agents.
        """
        ...

    def __repr__(self) -> str:
        """Return a string representation of the Principal client."""
        ...
