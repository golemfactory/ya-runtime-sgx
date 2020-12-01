import React from 'react';
import {Button, Modal, Form, Spinner} from 'react-bootstrap';
import {create_session} from '../Manager';

class NewVotingDialog extends React.Component {
    constructor(props) {
        super(props);
        this.state = {
            contract: "aea5db67524e02a263b9339fe6667d6b577f3d4c",
            votingId: "",
            pending: false
        };
        this.wrapper = React.createRef();
    }

    validate() {
        console.log('state', this.state);
        const {handleCreate} = this.props;
        const {contract, votingId} = this.state;

        this.setState({pending: true});
        create_session(contract, votingId).then(() => {
            this.setState({pending: false});
            handleCreate();
        })
        return false;
    }


    render() {
        const {handleClose,show} = this.props;
        const {contract, votingId, pending} = this.state;

        const handleSubmit = (e) => {
            e.preventDefault();
            this.validate()
        };

        return <Modal show={show} onHide={handleClose}>
            {pending ? "" : <Modal.Header closeButton>
                <Modal.Title>New Voting Session</Modal.Title>
            </Modal.Header> }
            { pending ? <Modal.Body>
                <Spinner animation="grow" variant="primary"/> Creating&configuring sgx enclave please wait.
            </Modal.Body> : <Form onSubmit={handleSubmit}>
            <Modal.Body>
                    <Form.Group>
                        <Form.Label>Contract:</Form.Label>
                        <Form.Control type="text" value={contract} onChange={(e) => this.setState({contract: e.target.value})}/>
                    </Form.Group>
                    <Form.Group className="error">
                        <Form.Label>Voting Id:</Form.Label>
                        <Form.Control type="text" value={votingId} onChange={(e) => this.setState({votingId: e.target.value})} />
                    </Form.Group>
            </Modal.Body>
             <Modal.Footer>
                 <Button variant="primary" type="submit" onSubmit={handleSubmit} disabled={pending}>
                     Save Changes
                 </Button>
             </Modal.Footer>
            </Form>}
        </Modal>;
    }
}

export default NewVotingDialog;