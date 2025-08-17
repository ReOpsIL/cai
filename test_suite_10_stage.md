# 10-Stage Web Application Test Suite

## Test Objective
Evaluate CAI's ability to incrementally build a complete web application through 10 sequential prompts, testing context retention, multi-stage development, and workflow continuity.

## Test Prompts Sequence

### Stage 1: Project Setup
**Prompt**: "Create a new React project called 'TaskManager' with TypeScript, set up the basic folder structure, and create a simple Hello World component."

### Stage 2: Routing Setup
**Prompt**: "Add React Router to the TaskManager project and create basic routes for Home, Tasks, and Settings pages."

### Stage 3: State Management
**Prompt**: "Implement Redux Toolkit for state management and create slices for tasks and user preferences."

### Stage 4: Task Component
**Prompt**: "Create a Task component that displays individual tasks with title, description, priority, and completion status."

### Stage 5: Task List
**Prompt**: "Build a TaskList component that renders multiple Task components and includes filtering by priority and completion status."

### Stage 6: Task Form
**Prompt**: "Create a TaskForm component for adding and editing tasks with validation and proper form handling."

### Stage 7: API Integration
**Prompt**: "Add API integration using axios to connect to a REST API for CRUD operations on tasks."

### Stage 8: Styling
**Prompt**: "Implement responsive CSS styling using Tailwind CSS or styled-components to make the application visually appealing."

### Stage 9: Testing
**Prompt**: "Add unit tests for the Task and TaskList components using Jest and React Testing Library."

### Stage 10: Deployment
**Prompt**: "Configure the project for deployment and create build scripts for production deployment."

## Expected Outcomes
Each stage should build upon the previous ones, maintaining context and integrating seamlessly with existing code.